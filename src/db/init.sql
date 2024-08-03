create table if not exists users(
    id integer primary key autoincrement,
    name text not null
);

create table if not exists messages(
    id integer primary key autoincrement,
    sender_id integer,
    content text,
    foreign key (user_id) references users(id)
)
