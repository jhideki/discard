create table if not exists users(
    user_id integer primary key autoincrement,
    display_name text not null,
    node_id blob not null
    status integer not null,
);

create table if not exists messages(
    message_id integer primary key autoincrement,
    sender_id integer,
    content text,
    received_ts text,
    sent_ts text,
    read_ts text,
    foreign key (sender_id) references users(user_id)
)
