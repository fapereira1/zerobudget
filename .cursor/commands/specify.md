# Create a feature specification

Input: Free‑form feature description (single paragraph is fine).
Side effects: Creates feature branch and specs/[###-feature-slug]/spec.md.

Execution Flow

Prepare feature context

Scan {repo}/specs/ for highest existing numeric prefix; pick next ### (zero‑padded).
Slugify feature name → ###-feature-slug.
Create specs/{###-feature-slug}/ and switch to branch feat/{###-feature-slug} (use Cursor’s Git panel; if Git is not initialized, add “INIT GIT” note).
Draft the spec

Populate from SPEC TEMPLATE (below).
Embed the original user input under Input.
Mark every ambiguity with [NEEDS CLARIFICATION: …].
Keep it business‑facing: what and outcomes, not implementation details.
Validate & save

Ensure User Scenarios & Testing and Requirements are complete.
For any vague requirement, add a clarification marker.
Save as specs/{###-feature-slug}/spec.md and stage changes.
Report

Print: feature branch, spec path, and any [NEEDS CLARIFICATION] items to resolve next.