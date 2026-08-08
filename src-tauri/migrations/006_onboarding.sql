-- First-run onboarding flag.
--
-- DEFAULT 1 (already onboarded) so that *existing* installs — whose database predates this
-- migration — are backfilled to 1 and never see the walkthrough or have their chosen theme
-- touched. A brand-new install can't be told apart from an old one in SQL alone (every
-- migration runs at first launch on a fresh DB), so the app flips this back to 0 and switches
-- the theme to Glass *only when it created the database this launch* — see `Database::new`'s
-- `is_fresh` handling. That keeps "Glass by default + show onboarding" for new users while
-- leaving returning users exactly as they were.
ALTER TABLE settings ADD COLUMN onboarded INTEGER NOT NULL DEFAULT 1;
