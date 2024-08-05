create table if not exists users(
    user_id integer primary key autoincrement,
    display_name text not null,
    node_id text not null
);

create table if not exists messages(
    message_id integer primary key autoincrement,
    sender_id integer,
    content text,
    foreign key (sender_id) references users(user_id)
)
