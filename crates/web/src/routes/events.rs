//! Event listing, detail and hangout creation.

use api::{CreateHangoutRequest, EventCard, EventDetail, EventListQuery, TagCount};
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use hangouts_core::{EventId, HangoutSpec, build_hangout, normalize_tags};
use store::EventFilter;

use super::event_id;
use crate::auth::{Auth, MaybeAuth};
use crate::error::{WebError, WebResult};
use crate::state::AppState;
use crate::view;

/// Default and maximum page size of `GET /api/events`.
const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;

/// `GET /api/events`.
pub async fn list(
    State(state): State<AppState>,
    MaybeAuth(viewer): MaybeAuth,
    Query(query): Query<EventListQuery>,
) -> WebResult<Json<Vec<EventCard>>> {
    let tags = query
        .tags
        .as_deref()
        .map(|t| normalize_tags(&t.split(',').collect::<Vec<_>>()))
        .unwrap_or_default();
    let filter = EventFilter {
        kind: query.kind,
        tags,
        from: Some(query.from.unwrap_or_else(|| state.now())),
        to: query.to,
        query: query.q.filter(|q| !q.trim().is_empty()),
        limit: Some(query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)),
    };
    let events = state.store.list_events(&filter).await?;
    let snap = state.store.load_event_view(&events, viewer).await?;
    Ok(Json(
        events
            .iter()
            .map(|e| view::card(&snap, e, viewer))
            .collect(),
    ))
}

/// `GET /api/events/{id}`.
pub async fn get(
    State(state): State<AppState>,
    MaybeAuth(viewer): MaybeAuth,
    Path(id): Path<String>,
) -> WebResult<Json<EventDetail>> {
    Ok(Json(detail_for(&state, event_id(&id)?, viewer).await?))
}

/// Loads the detail view of one event.
pub async fn detail_for(
    state: &AppState,
    id: EventId,
    viewer: Option<hangouts_core::UserId>,
) -> WebResult<EventDetail> {
    let event = state
        .store
        .get_event(id)
        .await?
        .ok_or_else(|| WebError::not_found("Event"))?;
    let snap = state
        .store
        .load_event_view(std::slice::from_ref(&event), viewer)
        .await?;
    Ok(view::detail(&snap, &event, viewer))
}

/// `POST /api/events`.
pub async fn create(
    State(state): State<AppState>,
    Auth(host): Auth,
    Json(req): Json<CreateHangoutRequest>,
) -> WebResult<(StatusCode, Json<EventDetail>)> {
    let spec = HangoutSpec::from(req);
    let now = state.now();
    let event = build_hangout(&spec, host, EventId::new(), now)?;
    if let Some(crew) = event.crew_id {
        let members = state.store.list_crew_members(crew).await?;
        if !members.iter().any(|m| m.user_id == host) {
            return Err(WebError::forbidden(
                "You can only host for crews you belong to.",
            ));
        }
    }
    state.store.insert_event(&event).await?;
    crate::embedding::embed_event(&state, &event).await;
    let detail = detail_for(&state, event.id, Some(host)).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

/// `GET /api/tags`.
pub async fn tags(State(state): State<AppState>) -> WebResult<Json<Vec<TagCount>>> {
    let tags = state.store.list_tags().await?;
    Ok(Json(
        tags.into_iter()
            .take(30)
            .map(|(tag, count)| TagCount { tag, count })
            .collect(),
    ))
}
