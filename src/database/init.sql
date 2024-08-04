create or replace table users(
    user_id integer primary key autoincrement,
    display_name text not null,
    node_id text not null
);

create or replace table messages(
    message_id integer primary key autoincrement,
    sender_id integer,
    content text,
    foreign key (user_id) references users(user_id)
)
