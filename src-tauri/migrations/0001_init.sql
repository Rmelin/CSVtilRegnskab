CREATE TABLE IF NOT EXISTS imported_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    imported_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    imported_file_id INTEGER NOT NULL,
    booking_date TEXT NOT NULL,
    value_date TEXT NOT NULL,
    text TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    balance NUMERIC NOT NULL,
    own_reference TEXT,
    suggested_budget_post_id INTEGER,
    assigned_budget_post_id INTEGER,
    matched_rule_id INTEGER,
    confirmed INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (imported_file_id) REFERENCES imported_files(id),
    FOREIGN KEY (suggested_budget_post_id) REFERENCES budget_posts(id),
    FOREIGN KEY (assigned_budget_post_id) REFERENCES budget_posts(id),
    FOREIGN KEY (matched_rule_id) REFERENCES matcher_rules(id),
    UNIQUE (booking_date, value_date, text, amount, balance, own_reference)
);

CREATE TABLE IF NOT EXISTS budget_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS budget_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (group_id) REFERENCES budget_groups(id)
);

CREATE TABLE IF NOT EXISTS matcher_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    regex_pattern TEXT NOT NULL,
    default_budget_post_id INTEGER,
    direction TEXT NOT NULL DEFAULT 'both',
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (default_budget_post_id) REFERENCES budget_posts(id)
);


CREATE INDEX IF NOT EXISTS idx_transactions_booking_date ON transactions(booking_date);
CREATE INDEX IF NOT EXISTS idx_transactions_confirmed ON transactions(confirmed);
CREATE INDEX IF NOT EXISTS idx_transactions_assignment ON transactions(assigned_budget_post_id);

INSERT INTO matcher_rules (name, regex_pattern, default_budget_post_id, direction, enabled, priority)
VALUES (
    'Kontingent - SE MEDD.',
    '^\s*(\d{6,10})\s+(.+?)\s+-SE\s+MEDD\.\s*$',
    NULL,
    'income',
    1,
    10
);
