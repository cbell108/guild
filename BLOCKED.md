# BLOCKED

Append under your own lane only.

## Lane A (web + store)

## Lane B (frontend)

## Lane C (recommend, llm, ingest, seed, eval, bot)

## Lane B
- B8 (partial): the host check-in page shows the rotating link with a copy button but draws no QR image — the frontend ships no dependencies and has no QR encoder. Attendees open the link (`/checkin?event=…&token=…`), which posts the token. A small vendored QR encoder (e.g. nayuki's MIT `qrcodegen`) would close this.
