CREATE TABLE IF NOT EXISTS budget_values (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    budget_post_id INTEGER NOT NULL,
    year INTEGER NOT NULL,
    amount TEXT,
    note TEXT,
    UNIQUE(budget_post_id, year),
    FOREIGN KEY (budget_post_id) REFERENCES budget_posts(id)
);
