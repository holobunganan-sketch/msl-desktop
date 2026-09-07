-- Preserve legacy combined institution text verbatim; users can clarify it later.
ALTER TABLE kol_experts ADD COLUMN department TEXT NOT NULL DEFAULT '';
