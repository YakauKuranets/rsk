# Coverage Matrix v1

Version: `coverage_matrix_v1`  
Date: `2026-03-26`

Machine-readable source: `docs/coverage_matrix_v1.json`.

## Purpose
This matrix is a **capability honesty map**. It records where current contours are strong, partial, weak, or none, with explicit false-safety zones.

## Coverage entries

| Contour | Capability | Coverage | Evidence quality | False-safety risk |
|---|---|---|---|---|
| archive_result_v1 | Archive discovery/export normalization (ISAPI/ONVIF/FTP/XM/HTTP) | partial | backed | Medium |
| auth_result_v1 | Auth/session posture normalization (HTTP/RTSP/ISAPI/ONVIF/FTP) | partial | backed | High |
| surface_scan_result_v1 + spider fingerprint | Surface discovery + fingerprint hints | partial | backed | High |
| spider evidence/baseline | Explainable spider evidence + known-bad baseline checks | partial | backed | Medium |
| port_scan_result_v1 | Port exposure normalization | strong | backed | Medium |
| port_audit_result_v1 | Risk audit over port scan | partial | backed | High |
| passive_observation_result_v1 | Passive traffic observation + correlation with surface | partial | indirect | High |
| session_lifecycle_known_bad_pack_v1 | Session/cookie lifecycle baseline scenarios | partial | backed | Medium |

## Strongest contours
- `port_scan_result_v1` is currently the strongest normalized contour.

## Weakest contours
- `passive_observation_result_v1` in low/no-traffic windows.
- Surface/spider fingerprint confidence when discovery signal is sparse.

## Top false-safety zones
1. Auth/session appears secure under fallback-limited evidence.
2. Fingerprint hints interpreted as definitive attribution.
3. Quiet passive windows interpreted as absence of risk.

## Known weak/none policy
- Weak/none are **not failures of reporting**; they are required truth signals for roadmap planning.
- Do not up-classify coverage from partial/weak without new evidence layers.

## Next priorities
- Phase 31.3 — Validation quality report layer.
- Confidence calibration improvements for sparse-signal contours.
- Confidence-to-action guardrails for operator workflows.
