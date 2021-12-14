FROM rust:1.57-buster

WORKDIR /project

COPY . .

RUN cargo install --path .

ENV ROCKET_ADDRESS=0.0.0.0

ENTRYPOINT ["backend"]

#ENTRYPOINT ["cargo", "run"]