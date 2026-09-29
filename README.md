# Notes Web

A lightweight notes app built with Rust, Axum, and PostgreSQL. Its responsive interface is inspired by Google Keep.

## Features

- Create notes with a title and multiline content
- Browse notes in a responsive, color-coded card layout
- Search notes by title or content
- Delete notes after confirming the action
- Store notes in PostgreSQL so they persist across app restarts and deployments

## Requirements

- Rust and Cargo
- A running PostgreSQL server and an existing database

## Run locally

Create a database named `notes` in PostgreSQL, then set `DATABASE_URL` to a connection URL for that database:

```sh
export DATABASE_URL='postgres://postgres:your_password@localhost:5432/notes'
cargo run
```

Replace `postgres` and `your_password` with your PostgreSQL username and password. If PostgreSQL runs in Docker with a published port, use the host and port exposed by the container (commonly `localhost:5432`).

The app listens on `0.0.0.0:3000` by default and creates the `notes` table automatically at startup. Set `PORT` to change the port.

Open <http://localhost:3000>.

## Deploy to Railway

1. Push this project to a GitHub repository and create a Railway project from that repository.
2. Add a PostgreSQL service to the Railway project.
3. In the web service variables, set `DATABASE_URL` to a reference to the PostgreSQL service, for example `${{Postgres.DATABASE_URL}}`. Replace `Postgres` with that service's name.
4. Deploy. Railway uses the included `Dockerfile`; the app reads Railway's `PORT` variable.
5. Generate a public domain for the web service in Railway's networking settings.

Notes are stored in PostgreSQL, not in the web container's filesystem.

## Security

This app currently has no user accounts or access controls. Anyone who can reach the app can view, create, and delete notes. Add authentication before exposing it publicly or storing private information.
