INSERT INTO matcher_rules (name, regex_pattern, default_budget_post_id, direction, enabled, priority)
VALUES
    ('Omkortninger', '(?i)^\s*Omkostninger\b.*$', NULL, 'expense', 1, 20),
    ('Holdsport', '^\s*BS HOLDSPORT\.DK APS\s*$', NULL, 'expense', 1, 30);
