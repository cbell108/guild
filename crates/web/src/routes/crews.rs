//! Crews, crew availability grids and the viewer's own availability.

use api::{
    AddCrewMemberRequest, AvailabilityView, CreateCrewRequest, CrewAvailability,
    CrewAvailabilityQuery, CrewDetail, CrewMemberView, CrewSummary, SetAvailabilityRequest,
    SlotView, UserSummary,
};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::TimeDelta;
use hangouts_core::time::{format_date, local_day, parse_date, parse_local_window};
use hangouts_core::{Availability, Crew, CrewId, CrewRole, UserId, best_slot, slots};

use super::parse_id;
use crate::auth::Auth;
use crate::error::{WebError, WebResult};
use crate::state::AppState;

const MAX_NAME_CHARS: usize = 60;
const DEFAULT_SLOT_MINUTES: u32 = 60;
const MIN_SLOT_MINUTES: u32 = 15;
const MAX_SLOT_MINUTES: u32 = 24 * 60;

fn summary(crew: &Crew) -> CrewSummary {
    CrewSummary {
        id: crew.id,
        name: crew.name.clone(),
        description: crew.description.clone(),
    }
}

/// Loads a crew the viewer belongs to: 404 if it doesn't exist, 403 for non-members.
async fn member_crew(state: &AppState, id: CrewId, uid: UserId) -> WebResult<Crew> {
    let crew = state
        .store
        .get_crew(id)
        .await?
        .ok_or_else(|| WebError::not_found("Crew"))?;
    let members = state.store.list_crew_members(id).await?;
    if members.iter().any(|m| m.user_id == uid) {
        Ok(crew)
    } else {
        Err(WebError::forbidden("Only crew members can see this crew."))
    }
}

/// Members, owners first then by join time.
async fn members_of(state: &AppState, crew: CrewId) -> WebResult<Vec<(UserSummary, CrewRole)>> {
    let mut members = state.store.list_crew_members(crew).await?;
    members.sort_by_key(|m| (m.role != CrewRole::Owner, m.joined_at, m.user_id));
    let mut out = Vec::new();
    for m in members {
        if let Some(user) = state.store.get_user(m.user_id).await? {
            out.push((UserSummary::from(&user), m.role));
        }
    }
    Ok(out)
}

async fn detail(state: &AppState, crew: &Crew) -> WebResult<CrewDetail> {
    let members = members_of(state, crew.id)
        .await?
        .into_iter()
        .map(|(user, role)| CrewMemberView { user, role })
        .collect();
    Ok(CrewDetail {
        crew: summary(crew),
        members,
    })
}

/// `GET /api/crews`: the viewer's crews.
pub async fn list(
    State(state): State<AppState>,
    Auth(uid): Auth,
) -> WebResult<Json<Vec<CrewSummary>>> {
    let crews = state.store.list_crews_for_user(uid).await?;
    Ok(Json(crews.iter().map(summary).collect()))
}

/// `POST /api/crews`.
pub async fn create(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Json(req): Json<CreateCrewRequest>,
) -> WebResult<(StatusCode, Json<CrewDetail>)> {
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(WebError::bad_request(format!(
            "Crew names must be 1 to {MAX_NAME_CHARS} characters."
        )));
    }
    let crew = Crew {
        id: CrewId::new(),
        name: name.to_owned(),
        description: req.description.trim().to_owned(),
        created_by: uid,
        created_at: state.now(),
    };
    state.store.insert_crew(&crew).await?;
    Ok((StatusCode::CREATED, Json(detail(&state, &crew).await?)))
}

/// `GET /api/crews/{id}`.
pub async fn get(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
) -> WebResult<Json<CrewDetail>> {
    let crew = member_crew(&state, parse_id(&id, "Crew")?, uid).await?;
    Ok(Json(detail(&state, &crew).await?))
}

/// Whether `uid` owns the crew.
async fn is_owner(state: &AppState, crew: CrewId, uid: UserId) -> WebResult<bool> {
    Ok(state
        .store
        .list_crew_members(crew)
        .await?
        .iter()
        .any(|m| m.user_id == uid && m.role == CrewRole::Owner))
}

/// `POST /api/crews/{id}/members`: owners add members (idempotent; never demotes an owner).
pub async fn add_member(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
    Json(req): Json<AddCrewMemberRequest>,
) -> WebResult<Json<CrewDetail>> {
    let crew = member_crew(&state, parse_id(&id, "Crew")?, uid).await?;
    if !is_owner(&state, crew.id, uid).await? {
        return Err(WebError::forbidden("Only the crew owner can add members."));
    }
    let existing = state.store.list_crew_members(crew.id).await?;
    if !existing.iter().any(|m| m.user_id == req.user_id) {
        state
            .store
            .add_crew_member(crew.id, req.user_id, CrewRole::Member, state.now())
            .await
            .map_err(|e| match e {
                store::StoreError::NotFound => WebError::not_found("User"),
                other => other.into(),
            })?;
    }
    Ok(Json(detail(&state, &crew).await?))
}

/// `DELETE /api/crews/{id}/members/{user_id}`: owners remove others; anyone may leave.
/// When the last owner leaves, the longest-standing member becomes the owner.
pub async fn remove_member(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path((id, target)): Path<(String, String)>,
) -> WebResult<Json<CrewDetail>> {
    let crew = member_crew(&state, parse_id(&id, "Crew")?, uid).await?;
    let target: UserId = parse_id(&target, "User")?;
    if target != uid && !is_owner(&state, crew.id, uid).await? {
        return Err(WebError::forbidden(
            "Only the crew owner can remove other members.",
        ));
    }
    state.store.remove_crew_member(crew.id, target).await?;
    let mut remaining = state.store.list_crew_members(crew.id).await?;
    if !remaining.is_empty() && !remaining.iter().any(|m| m.role == CrewRole::Owner) {
        remaining.sort_by_key(|m| (m.joined_at, m.user_id));
        state
            .store
            .add_crew_member(crew.id, remaining[0].user_id, CrewRole::Owner, state.now())
            .await?;
    }
    Ok(Json(detail(&state, &crew).await?))
}

/// `GET /api/crews/{id}/availability?date=`: the crew's free-time grid for one local day.
pub async fn availability(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Path(id): Path<String>,
    Query(query): Query<CrewAvailabilityQuery>,
) -> WebResult<Json<CrewAvailability>> {
    let crew = member_crew(&state, parse_id(&id, "Crew")?, uid).await?;
    let date = parse_date(&query.date)?;
    let slot_minutes = query
        .slot_minutes
        .unwrap_or(DEFAULT_SLOT_MINUTES)
        .clamp(MIN_SLOT_MINUTES, MAX_SLOT_MINUTES);
    let members = members_of(&state, crew.id).await?;
    let ids: Vec<UserId> = members.iter().map(|(u, _)| u.id).collect();
    let entries = state.store.list_availability_on(date, Some(&ids)).await?;
    let grid = slots(
        &entries,
        local_day(date),
        TimeDelta::minutes(i64::from(slot_minutes)),
    );
    let view = |s: &hangouts_core::Slot| SlotView {
        starts_at: s.window.start(),
        ends_at: s.window.end(),
        available: s.available.clone(),
    };
    Ok(Json(CrewAvailability {
        date: format_date(date),
        slot_minutes,
        members: members.into_iter().map(|(u, _)| u).collect(),
        slots: grid.iter().map(view).collect(),
        best: best_slot(&grid).map(view),
    }))
}

/// `PUT /api/me/availability`: one window per local date, like the bot's `/availability`.
pub async fn set_availability(
    State(state): State<AppState>,
    Auth(uid): Auth,
    Json(req): Json<SetAvailabilityRequest>,
) -> WebResult<Json<AvailabilityView>> {
    let date = parse_date(&req.date)?;
    let window = parse_local_window(&req.date, req.start.as_deref(), req.end.as_deref())?;
    state
        .store
        .set_availability(&Availability {
            user_id: uid,
            window,
        })
        .await?;
    Ok(Json(AvailabilityView {
        date: format_date(date),
        starts_at: window.start(),
        ends_at: window.end(),
        overnight: window.crosses_midnight(),
    }))
}
