// Builds campus-hangouts.pptx. Run: NODE_PATH=<dir with pptxgenjs> node docs/deck/build.js
const pptxgen = require("pptxgenjs");
const path = require("path");
const pres = new pptxgen();
pres.layout = "LAYOUT_16x9"; // 10 x 5.625 in
pres.title = "campus-hangouts";
pres.theme = { headFontFace: "Cambria", bodyFontFace: "Calibri" };

const INK = "1A1A1A", GOLD = "FDBB30", PAPER = "FFFFFF", SOFT = "F3F3F1", MUTED = "5F5F5F", GREEN = "2E7D5B";
const A = (f) => path.join(__dirname, "assets", f);

pres.defineSlideMaster({
  title: "LIGHT", background: { color: PAPER },
  slideNumber: { x: 9.2, y: 5.2, w: 0.5, h: 0.3, fontFace: "Calibri", fontSize: 10, color: MUTED },
  objects: [{ placeholder: { options: { name: "title", type: "title", x: 0.6, y: 0.4, w: 8.8, h: 0.8, fontFace: "Cambria", fontSize: 30, bold: true, color: INK, align: "left", valign: "top", margin: 0 }, text: "" } }],
});
pres.defineSlideMaster({
  title: "DARK", background: { color: INK },
  objects: [{ placeholder: { options: { name: "title", type: "title", x: 0.6, y: 0.4, w: 8.8, h: 0.8, fontFace: "Cambria", fontSize: 30, bold: true, color: PAPER, align: "left", valign: "top", margin: 0 }, text: "" } }],
});

function slide(master, title, notes) {
  const s = pres.addSlide({ masterName: master });
  s.addText(title, { placeholder: "title" });
  s.addNotes(notes);
  return s;
}
const T = (s, text, o) => s.addText(text, Object.assign({ isTextBox: true, fontFace: "Calibri", margin: 0, color: INK }, o));
function card(s, x, y, w, h, fill) {
  s.addShape(pres.ShapeType.roundRect, { x, y, w, h, rectRadius: 0.08, fill: { color: fill || SOFT }, line: { color: fill || SOFT } });
}

// 1 Title
{
  const s = pres.addSlide({ masterName: "DARK" });
  T(s, "campus-hangouts", { x: 0.6, y: 1.5, w: 8.8, h: 1.0, fontFace: "Cambria", fontSize: 54, bold: true, color: PAPER });
  T(s, "Never go alone.", { x: 0.6, y: 2.6, w: 8.8, h: 0.6, fontSize: 28, color: GOLD, italic: true });
  T(s, "Events and hangouts for KSU, with someone to go with", { x: 0.6, y: 3.3, w: 8.8, h: 0.4, fontSize: 18, color: "DDDDDD" });
  T(s, "KSU AI Club Hackathon 2026  ·  [TEAM NAMES]", { x: 0.6, y: 4.7, w: 8.8, h: 0.3, fontSize: 14, color: "BBBBBB" });
  s.addNotes("We built campus-hangouts: a web app that finds you the right campus event, shows which friends are going, and finds someone to go with. (10 s)");
}

// 2 Problem
{
  const s = slide("LIGHT", "Platforms assume you go alone",
    "64% of about 48,000 students felt very lonely in the past year. About a quarter struggle with social anxiety. Free events routinely see over 50% no-shows. So going alone is the real barrier, and RSVPs mean little. Meanwhile discovery is a date-ordered firehose and informal hangouts never appear. (25 s)");
  const stats = [
    ["64%", "of ~48,000 students felt very lonely in the past year", "Chapman University news"],
    ["25.8%", "struggle with social anxiety", "Talkspace"],
    [">50%", "no-shows at free events, vs 10–20% for paid", "AddEvent"],
  ];
  stats.forEach((st, i) => {
    const x = 0.6 + i * 3.0;
    card(s, x, 1.5, 2.8, 2.5);
    T(s, st[0], { x: x + 0.2, y: 1.7, w: 2.4, h: 0.9, fontFace: "Cambria", fontSize: 48, bold: true });
    T(s, st[1], { x: x + 0.2, y: 2.65, w: 2.4, h: 0.8, fontSize: 15 });
    T(s, "Source: " + st[2], { x: x + 0.2, y: 3.55, w: 2.4, h: 0.3, fontSize: 11, color: MUTED });
  });
  T(s, "Discovery is a date-ordered firehose, and informal hangouts never appear at all.", { x: 0.6, y: 4.35, w: 8.8, h: 0.5, fontSize: 16, color: MUTED });
}

// 3 Who it's for
{
  const s = slide("LIGHT", "Who it's for",
    "Students, especially newcomers and commuters; student hosts who aren't an official org; and organizations. KSU's calendar is mid-migration to Owl Life, and the public calendar API gives us official events to blend with student hangouts. (15 s)");
  const rows = [
    ["Students", "Find events worth going to, and someone to go with"],
    ["Student hosts", "Anyone can host a hangout, no official org needed"],
    ["Organizations", "Official KSU events from the public calendar API, in the same feed"],
  ];
  rows.forEach((r, i) => {
    const y = 1.5 + i * 1.15;
    card(s, 0.6, y, 8.8, 0.95);
    s.addShape(pres.ShapeType.ellipse, { x: 0.85, y: y + 0.22, w: 0.5, h: 0.5, fill: { color: GOLD }, line: { color: GOLD } });
    T(s, String(i + 1), { x: 0.85, y: y + 0.22, w: 0.5, h: 0.5, align: "center", valign: "middle", bold: true, fontSize: 18 });
    T(s, r[0], { x: 1.65, y: y + 0.12, w: 2.4, h: 0.7, valign: "middle", bold: true, fontSize: 20, fontFace: "Cambria" });
    T(s, r[1], { x: 4.1, y: y + 0.12, w: 5.1, h: 0.7, valign: "middle", fontSize: 16 });
  });
}

// 4 Solution
{
  const s = slide("LIGHT", "One feed, with someone to go with",
    "The For you feed puts official events and student hangouts together, with a one-line reason on every card, and it's finite: 'You're caught up'. Intent has three levels, with a one-tap back-out and waitlist auto-promote. Buddy matching finds a friend first, else a similar student. Anyone can host, including by describing it in plain English to a local LLM. Crews share an availability grid. (30 s)");
  const tiles = [
    ["For you feed", "A plain-English reason on every card. Finite by design"],
    ["Interested → Going → Going with", "Waitlist auto-promotes; one-tap back-out"],
    ["Buddy matching", "A friend first, else a similar student"],
    ["Friend presence", "See which friends are going. Never full lists"],
    ["Anyone can host", "Describe it in plain English; a local LLM fills the form"],
    ["Crews", "Small groups with a shared availability grid"],
  ];
  tiles.forEach((t, i) => {
    const x = 0.6 + (i % 3) * 3.0, y = 1.4 + Math.floor(i / 3) * 1.85;
    card(s, x, y, 2.8, 1.65);
    T(s, t[0], { x: x + 0.2, y: y + 0.15, w: 2.4, h: 0.55, bold: true, fontSize: 16, fontFace: "Cambria", valign: "top" });
    T(s, t[1], { x: x + 0.2, y: y + 0.8, w: 2.4, h: 0.75, fontSize: 14, color: MUTED, valign: "top" });
  });
}

// 5 Product screenshot
{
  const s = slide("LIGHT", "The For you feed",
    "This is the real app with seeded demo students. Official KSU events and student hangouts sit together; each card has a reason, friends going, and spots left. (15 s)");
  s.addImage({ path: A("feed-for-you-desktop.jpg"), x: 0.6, y: 1.35, w: 5.6, h: 3.5, altText: "Screenshot of the For you feed" });
  const pts = ["Official KSU events and student hangouts side by side", "A reason on every card, plus 'Why am I seeing this?'", "Friends going, spots left", "Finite: 'You're caught up'"];
  T(s, pts.map((p, i) => ({ text: p, options: { bullet: true, breakLine: i < pts.length - 1, paraSpaceAfter: 10 } })), { x: 6.55, y: 1.45, w: 2.9, h: 3.4, fontSize: 15, valign: "top" });
}

// 5b Phone screenshots
{
  const s = slide("LIGHT", "Works on a phone, too",
    "The same app on a phone: going with a friend and finding a buddy, hosting by describing the plan, and privacy settings. All real screenshots of the running app with seeded demo data. (10 s)");
  const shots = [["event-going-with-buddy-mobile.jpg", 2174, "Going with + buddy match"], ["host-describe-prefill-mobile.jpg", 1688, "Describe it, we prefill"], ["privacy-settings-mobile.jpg", 1688, "Privacy controls"]];
  shots.forEach((sh, i) => {
    const w = 1.66, fullH = w * sh[1] / 780, x = 1.2 + i * 2.8;
    s.addImage({ path: A(sh[0]), x, y: 1.3, w, h: fullH, sizing: { type: "crop", x: 0, y: 0, w, h: 3.2 }, altText: sh[2] });
    T(s, sh[2], { x: x - 0.4, y: 4.65, w: 2.5, h: 0.3, fontSize: 13, color: MUTED, align: "center" });
  });
}

// 6 Live demo
{
  const s = pres.addSlide({ masterName: "DARK" });
  s.addText("Live demo", { placeholder: "title" });
  s.addNotes("Hand over to the demo. Follow docs/DEMO.md. If the LLM is down, fill the form by hand; if the backend is down, open /?mock=1. (3 min, not counted in the 4)");
  const steps = ["For you feed and reasons", "Going with + buddy match", "Host: describe it", "Privacy controls"];
  steps.forEach((t, i) => {
    const x = 0.6 + i * 2.25;
    s.addShape(pres.ShapeType.ellipse, { x, y: 2.0, w: 0.7, h: 0.7, fill: { color: GOLD }, line: { color: GOLD } });
    T(s, String(i + 1), { x, y: 2.0, w: 0.7, h: 0.7, align: "center", valign: "middle", bold: true, fontSize: 24 });
    T(s, t, { x, y: 2.9, w: 2.0, h: 0.9, fontSize: 17, color: PAPER, valign: "top" });
  });
  T(s, "Fallback: /?mock=1 runs on static mock data", { x: 0.6, y: 4.5, w: 8.8, h: 0.3, fontSize: 13, color: "BBBBBB" });
}

// 7 Recommender
{
  const s = slide("LIGHT", "How the recommender works",
    "The score is a weighted sum of three explainable parts. Content uses embeddings of event text versus a student's interests and history, which handles cold start. Collaborative is item-item over implicit feedback, weighting check-in above going above interested. Social boosts events friends are going to and uses a learned probability that an RSVP becomes a real attendance. Every part produces the reason line. (25 s)");
  const parts = [["Content", "Embeddings of event text vs. interests and history. Handles cold start"], ["Collaborative", "Item-item over implicit feedback: check-in > going > interested"], ["Social", "Friend boost and learned chance you'll actually show up"]];
  parts.forEach((p, i) => {
    const x = 0.6 + i * 3.0;
    card(s, x, 1.5, 2.8, 2.2);
    T(s, p[0], { x: x + 0.2, y: 1.65, w: 2.4, h: 0.5, bold: true, fontSize: 22, fontFace: "Cambria" });
    T(s, p[1], { x: x + 0.2, y: 2.3, w: 2.4, h: 1.3, fontSize: 15, valign: "top" });
  });
  card(s, 0.6, 4.0, 8.8, 0.9, GOLD);
  T(s, "Hybrid score = w1·content + w2·collaborative + w3·social  →  one-line reason on every card", { x: 0.85, y: 4.0, w: 8.3, h: 0.9, fontSize: 18, bold: true, valign: "middle" });
}

// 8 Results
{
  const s = slide("LIGHT", "Hybrid beats popularity 3.2×",
    "Offline evaluation: last week held out, the answer key is real check-ins, not RSVPs, averaged over 110 student-weeks and 5 seeds. Hybrid precision at 5 is 0.189 versus 0.059 for popularity, a 3.18 times lift. Recall at 10 is 0.883 versus 0.558. NDCG at 10 is 0.618 versus 0.255. The attendance model cuts log loss from 0.685 to 0.554. Caveat: this is seeded synthetic demo data, not real students. (25 s)");
  s.addChart(pres.charts.BAR, [
    { name: "Popularity baseline", labels: ["Precision@5", "Recall@10", "NDCG@10"], values: [0.059, 0.558, 0.255] },
    { name: "Hybrid", labels: ["Precision@5", "Recall@10", "NDCG@10"], values: [0.189, 0.883, 0.618] },
  ], {
    x: 0.6, y: 1.3, w: 5.8, h: 3.6, barDir: "col", barGrouping: "clustered", chartColors: ["B5B5B5", GOLD],
    showLegend: true, legendPos: "b", legendFontSize: 12, legendFontFace: "Calibri", legendColor: INK,
    showValue: true, dataLabelPosition: "outEnd", dataLabelFormatCode: "0.000", dataLabelFontSize: 11, dataLabelColor: INK, dataLabelFontFace: "Calibri",
    catAxisLabelColor: INK, catAxisLabelFontSize: 12, catAxisLabelFontFace: "Calibri",
    valAxisLabelColor: MUTED, valAxisLabelFontSize: 11, valAxisLabelFontFace: "Calibri", valAxisMaxVal: 1, valAxisMinVal: 0,
    valGridLine: { color: "E3E3E3", size: 0.5 }, catGridLine: { style: "none" },
  });
  card(s, 6.7, 1.4, 2.7, 1.5);
  T(s, "×3.18", { x: 6.9, y: 1.5, w: 2.3, h: 0.8, fontFace: "Cambria", fontSize: 44, bold: true });
  T(s, "precision@5 vs popularity", { x: 6.9, y: 2.3, w: 2.3, h: 0.5, fontSize: 14, color: MUTED });
  card(s, 6.7, 3.1, 2.7, 1.8);
  T(s, "×1.58", { x: 6.9, y: 3.2, w: 2.3, h: 0.8, fontFace: "Cambria", fontSize: 44, bold: true });
  T(s, "recall@10 vs popularity. Seeded synthetic data; real check-ins as the answer key", { x: 6.9, y: 4.0, w: 2.3, h: 0.8, fontSize: 12, color: MUTED, valign: "top" });
}

// 9 Privacy
{
  const s = slide("LIGHT", "Privacy by design",
    "Students told us they feel tracked, not helped, so these rules are normative in our API contract and tested. There are no attendee lists or counts. Friends see friends only, and only those who opted in. Attendance is private by default. Others only ever see id, name and bio. Buddy matching is opt-in, personalization can be turned off, crews are members-only, and history is deletable. (25 s)");
  const rules = [
    ["No attendee lists", "No endpoint returns who's going or a total count"],
    ["Friends only", "Counted only if accepted and they opted in"],
    ["Private by default", "Attendance hidden until students choose otherwise"],
    ["Minimal profile", "Others see only name and bio, never interests or settings"],
    ["Opt-in buddy matching", "Only users looking for a buddy are suggested"],
    ["Your data, your call", "Turn off personalization; delete your history"],
  ];
  rules.forEach((r, i) => {
    const x = 0.6 + (i % 2) * 4.5, y = 1.4 + Math.floor(i / 2) * 1.2;
    card(s, x, y, 4.3, 1.05);
    s.addShape(pres.ShapeType.ellipse, { x: x + 0.2, y: y + 0.3, w: 0.45, h: 0.45, fill: { color: GREEN }, line: { color: GREEN } });
    T(s, "✓", { x: x + 0.2, y: y + 0.3, w: 0.45, h: 0.45, align: "center", valign: "middle", color: PAPER, bold: true, fontSize: 16 });
    T(s, r[0], { x: x + 0.85, y: y + 0.1, w: 3.3, h: 0.35, bold: true, fontSize: 16, fontFace: "Cambria" });
    T(s, r[1], { x: x + 0.85, y: y + 0.48, w: 3.3, h: 0.5, fontSize: 13, color: MUTED, valign: "top" });
  });
}

// 10 Engineering
{
  const s = slide("DARK", "Engineering and organization",
    "For the Nudox track: ten crates with a strict dependency graph, and the recommender never touches the database so it is unit-testable. Core and API contracts were frozen early with mock JSON that round-trips through the wire types. 184 tests, with fmt and clippy as errors enforced by a pre-commit hook. And a docs set: spec, API contract, decisions, eval, demo script. (25 s)");
  const crates = ["core", "api", "store", "recommend", "llm", "ingest", "seed", "eval", "web", "bot"];
  crates.forEach((c, i) => {
    const x = 0.6 + (i % 5) * 1.05, y = 1.5 + Math.floor(i / 5) * 0.8;
    s.addShape(pres.ShapeType.roundRect, { x, y, w: 1.0, h: 0.6, rectRadius: 0.06, fill: { color: "333333" }, line: { color: GOLD, width: 1 } });
    T(s, c, { x, y, w: 1.0, h: 0.6, align: "center", valign: "middle", fontSize: 11, color: PAPER });
  });
  T(s, "10 Rust crates, no dependency cycles", { x: 0.6, y: 3.15, w: 4.9, h: 0.3, fontSize: 13, color: "BBBBBB" });
  const stats = [["184", "tests"], ["2", "frozen contracts"], ["5", "design docs"]];
  stats.forEach((st, i) => {
    const x = 0.6 + i * 1.7;
    T(s, st[0], { x, y: 3.7, w: 1.5, h: 0.7, fontFace: "Cambria", fontSize: 40, bold: true, color: GOLD });
    T(s, st[1], { x, y: 4.4, w: 1.5, h: 0.3, fontSize: 13, color: "DDDDDD" });
  });
  const pts = ["Recommender is DB-free, so it's unit-testable", "Core and API frozen; mock JSON round-trips through wire types", "fmt + clippy -D warnings via pre-commit hook", "SPEC, API, DECISIONS, EVAL and DEMO docs"];
  T(s, pts.map((p, i) => ({ text: p, options: { bullet: true, breakLine: i < pts.length - 1, paraSpaceAfter: 12 } })), { x: 5.9, y: 1.5, w: 3.5, h: 3.2, fontSize: 15, color: PAPER, valign: "top" });
}

// 11 What's next
{
  const s = slide("LIGHT", "What's next",
    "Next: offline-capable QR check-in, attendee-only threads after an event, club streaks rather than individual ones, a pilot with KSU organizations, and re-running the evaluation on real data. (10 s)");
  const items = ["Offline-capable QR check-in", "After-event attendee-only threads", "Club (not individual) streaks", "Pilot with KSU organizations", "Re-run the evaluation on real data"];
  items.forEach((t, i) => {
    const y = 1.4 + i * 0.7;
    s.addShape(pres.ShapeType.ellipse, { x: 0.6, y: y + 0.05, w: 0.45, h: 0.45, fill: { color: GOLD }, line: { color: GOLD } });
    T(s, String(i + 1), { x: 0.6, y: y + 0.05, w: 0.45, h: 0.45, align: "center", valign: "middle", bold: true, fontSize: 15 });
    T(s, t, { x: 1.3, y, w: 7.5, h: 0.55, valign: "middle", fontSize: 20 });
  });
}

// 12 Thanks
{
  const s = pres.addSlide({ masterName: "DARK" });
  T(s, "Thank you", { x: 0.6, y: 1.7, w: 8.8, h: 1.0, fontFace: "Cambria", fontSize: 54, bold: true, color: PAPER });
  T(s, "Never go alone.", { x: 0.6, y: 2.8, w: 8.8, h: 0.6, fontSize: 28, color: GOLD, italic: true });
  T(s, "[TEAM NAMES]  ·  Questions?", { x: 0.6, y: 4.5, w: 8.8, h: 0.4, fontSize: 18, color: "DDDDDD" });
  s.addNotes("Thank the judges, introduce the team, take questions. (5 s)");
}

pres.writeFile({ fileName: path.join(__dirname, "campus-hangouts.pptx") }).then(() => console.log("written"));
