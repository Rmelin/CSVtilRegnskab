CREATE TABLE budget_posts_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    post_type TEXT NOT NULL DEFAULT 'expense',
    FOREIGN KEY (group_id) REFERENCES budget_groups(id)
);

INSERT INTO budget_posts_new (id, group_id, name, sort_order, post_type)
SELECT id, group_id, name, sort_order, post_type FROM budget_posts;

DROP TABLE budget_posts;

ALTER TABLE budget_posts_new RENAME TO budget_posts;
