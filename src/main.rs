use axum::{
    Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sqlx::{FromRow, PgPool, postgres::PgPoolOptions};
use std::{env, error::Error, net::SocketAddr};

#[derive(FromRow)]
struct Note {
    id: i64,
    title: String,
    content: String,
}

#[derive(Deserialize)]
struct NewNote {
    title: String,
    content: String,
}

struct AppError(sqlx::Error);

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("Database request failed: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong while accessing notes.",
        )
            .into_response()
    }
}

async fn index(State(pool): State<PgPool>) -> Result<Html<String>, AppError> {
    let notes = sqlx::query_as::<_, Note>("SELECT id, title, content FROM notes ORDER BY id DESC")
        .fetch_all(&pool)
        .await?;

    Ok(Html(render_page(&notes)))
}

async fn create_note(
    State(pool): State<PgPool>,
    Form(form): Form<NewNote>,
) -> Result<Response, AppError> {
    let title = form.title.trim();
    if title.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Html(render_message(
                "The title cannot be empty.",
                &form.title,
                &form.content,
            )),
        )
            .into_response());
    }

    sqlx::query("INSERT INTO notes (title, content) VALUES ($1, $2)")
        .bind(title)
        .bind(form.content)
        .execute(&pool)
        .await?;

    Ok(Redirect::to("/").into_response())
}

async fn delete_note(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Redirect, AppError> {
    sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;

    Ok(Redirect::to("/"))
}

fn render_page(notes: &[Note]) -> String {
    let notes_html = if notes.is_empty() {
        "<p class=\"empty\">No notes yet. Add your first note above.</p>".to_owned()
    } else {
        notes
            .iter()
            .map(|note| {
                format!(
                    r#"<article class="note note-color-{}">
  <h2>{}</h2>
  <p>{}</p>
  <form action="/notes/{}/delete" method="post" class="delete-form" onsubmit="return confirm('Are you sure to remote it?')">
    <button class="delete" type="submit" aria-label="Delete {}" title="Delete note">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 7h12M10 11v6m4-6v6M8 7l1 13h6l1-13M9 7V4h6v3"/></svg>
    </button>
  </form>
</article>"#,
                    note.id.rem_euclid(6),
                    escape_html(&note.title),
                    escape_html(&note.content),
                    note.id,
                    escape_html(&note.title),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    render_document(None, notes_html)
}

fn render_message(message: &str, title: &str, content: &str) -> String {
    let form = render_composer(Some(message), title, content, true);
    render_document(
        Some(form),
        "<p class=\"empty\">Your note was not saved.</p>".to_owned(),
    )
}

fn render_composer(error: Option<&str>, title: &str, content: &str, expanded: bool) -> String {
    let open_class = if expanded { " is-open" } else { "" };
    let error_html = error
        .map(|message| format!(r#"<p class="error">{}</p>"#, escape_html(message)))
        .unwrap_or_default();
    format!(
        r#"<form class="composer{}" action="/notes" method="post">
  {}
  <input id="title" name="title" maxlength="200" required autocomplete="off" placeholder="Take a note..." value="{}" aria-label="Title">
  <textarea id="content" name="content" rows="3" placeholder="Write your note..." aria-label="Content">{}</textarea>
  <div class="composer-actions">
    <span class="composer-hint">Notes are saved to your collection</span>
    <button class="save-button" type="submit">Done</button>
  </div>
</form>"#,
        open_class,
        error_html,
        escape_html(title),
        escape_html(content),
    )
}

fn render_document(replacement_form: Option<String>, notes_html: String) -> String {
    let form = replacement_form.unwrap_or_else(|| render_composer(None, "", "", false));
    let notes_empty = notes_html.contains("class=\"empty\"");
    let empty_class = if notes_empty { " is-empty" } else { "" };
    let page = r###"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Notes</title>
  <style>
    :root { color-scheme: light; font-family: Arial, sans-serif; color: #202124; background: #fff; font-synthesis: none; }
    * { box-sizing: border-box; }
    body { margin: 0; min-width: 320px; }
    button, input, textarea { font: inherit; }
    .topbar { height: 65px; display: flex; align-items: center; gap: 28px; padding: 0 24px; border-bottom: 1px solid #dadce0; position: sticky; top: 0; z-index: 2; background: #fff; }
    .brand { display: flex; align-items: center; gap: 10px; flex: 0 0 218px; font-size: 22px; color: #5f6368; }
    .brand svg { width: 34px; height: 34px; }
    .search { width: min(720px, 70vw); height: 48px; border-radius: 9px; display: flex; align-items: center; gap: 14px; padding: 0 16px; background: #f1f3f4; }
    .search svg, .side-icon, .delete svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
    .search svg { width: 22px; height: 22px; color: #5f6368; flex: none; }
    .search input { border: 0; outline: 0; width: 100%; background: transparent; font-size: 16px; color: #202124; }
    .layout { display: flex; min-height: calc(100vh - 65px); }
    .sidebar { width: 256px; padding: 18px 12px; flex: none; }
    .nav-item { height: 48px; display: flex; align-items: center; gap: 20px; padding: 0 24px; border-radius: 0 24px 24px 0; color: #3c4043; text-decoration: none; font-size: 14px; font-weight: 500; }
    .nav-item.active { background: #feefc3; color: #202124; }
    .side-icon { width: 20px; height: 20px; color: #5f6368; }
    main { flex: 1; min-width: 0; padding: 34px clamp(20px, 5vw, 76px) 60px; }
    .composer { position: relative; width: min(600px, 100%); margin: 0 auto 42px; border: 1px solid #e0e0e0; border-radius: 8px; box-shadow: 0 2px 5px #0000001a; background: white; padding: 0 16px; transition: box-shadow .16s ease; }
    .composer:focus-within { box-shadow: 0 3px 8px #00000026; }
    .composer input, .composer textarea { display: block; width: 100%; border: 0; outline: 0; resize: vertical; color: #202124; background: transparent; }
    .composer input { height: 52px; font-weight: 500; }
    .composer textarea { display: none; min-height: 74px; padding: 5px 0; line-height: 1.5; }
    .composer.is-open textarea { display: block; }
    .composer-actions { display: none; align-items: center; justify-content: space-between; padding: 8px 0 10px; }
    .composer.is-open .composer-actions { display: flex; }
    .composer-hint { font-size: 12px; color: #80868b; }
    .save-button { border: 0; border-radius: 4px; background: transparent; padding: 8px 18px; color: #202124; font-weight: 500; cursor: pointer; }
    .save-button:hover { background: #f1f3f4; }
    .error { color: #b3261e; font-size: 13px; margin: 12px 0 0; }
    .section-label { margin: 0 auto 14px; max-width: 1000px; text-transform: uppercase; letter-spacing: .08em; font-size: 11px; font-weight: 600; color: #5f6368; }
    .notes-grid { columns: 4 220px; column-gap: 16px; max-width: 1000px; margin: 0 auto; }
    .note { display: inline-block; width: 100%; break-inside: avoid; position: relative; margin: 0 0 16px; padding: 16px 16px 40px; border: 1px solid #e0e0e0; border-radius: 8px; background: white; transition: box-shadow .16s ease; }
    .note:hover { box-shadow: 0 2px 8px #0000002b; }
    .note h2 { margin: 0 0 10px; font-size: 16px; line-height: 1.4; font-weight: 600; overflow-wrap: anywhere; }
    .note p { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font-size: 14px; line-height: 1.5; color: #3c4043; }
    .note-color-1 { background: #f28b82; border-color: #f28b82; }
    .note-color-2 { background: #fbbc04; border-color: #fbbc04; }
    .note-color-3 { background: #fff475; border-color: #fff475; }
    .note-color-4 { background: #ccff90; border-color: #ccff90; }
    .note-color-5 { background: #a7ffeb; border-color: #a7ffeb; }
    .delete-form { position: absolute; right: 8px; bottom: 6px; opacity: 0; transition: opacity .15s ease; }
    .note:hover .delete-form, .delete-form:focus-within { opacity: 1; }
    .delete { border: 0; border-radius: 50%; width: 34px; height: 34px; display: grid; place-items: center; color: #5f6368; background: transparent; cursor: pointer; }
    .delete:hover { background: #00000012; color: #202124; }
    .delete svg { width: 18px; height: 18px; }
    .empty { color: #80868b; text-align: center; padding: 35px 12px; font-size: 14px; }
    .empty-icon { display: block; width: 56px; height: 56px; margin: 0 auto 14px; color: #dadce0; }
    @media (max-width: 1100px) { .notes-grid { columns: 3 210px; } }
    @media (max-width: 850px) { .brand { flex-basis: auto; } .sidebar { width: 80px; padding-inline: 8px; } .nav-item { justify-content: center; padding: 0; font-size: 0; } main { padding-inline: 24px; } .notes-grid { columns: 2 210px; } }
    @media (max-width: 560px) { .topbar { height: 58px; padding: 0 12px; gap: 12px; } .brand { gap: 6px; font-size: 18px; } .brand svg { width: 30px; height: 30px; } .search { height: 42px; width: auto; flex: 1; padding: 0 10px; } .search input { font-size: 14px; } .layout { min-height: calc(100vh - 58px); } .sidebar { width: 56px; padding: 16px 4px; } .nav-item { height: 42px; } main { padding: 24px 14px; } .notes-grid { columns: 1; } .delete-form { opacity: 1; } }
  </style>
</head>
<body>
  <header class="topbar">
    <div class="brand">
      <svg viewBox="0 0 48 48" aria-hidden="true"><path fill="#fbbc04" d="M24 5a13 13 0 0 0-8 23.25V34h16v-5.75A13 13 0 0 0 24 5Z"/><path fill="#fbbc04" d="M18 37h12v3H18zm3 6h6v2h-6z"/><path fill="#fff" d="M22 13h4v8h-4z"/></svg>
      <span>Notes</span>
    </div>
    <label class="search">
      <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="10.8" cy="10.8" r="6.8"/><path d="m16 16 5 5"/></svg>
      <input id="note-search" type="search" placeholder="Search your notes" aria-label="Search your notes">
    </label>
  </header>
  <div class="layout">
    <aside class="sidebar">
      <a class="nav-item active" href="/" aria-current="page">
        <svg class="side-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4.5h14a1.5 1.5 0 0 1 1.5 1.5v12a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 18V6A1.5 1.5 0 0 1 5 4.5Z"/><path d="M7.5 9h9m-9 3.5h9m-9 3.5h5"/></svg>
        Notes
      </a>
    </aside>
    <main>
      __FORM__
      <section aria-labelledby="saved-notes">
        <h1 class="section-label" id="saved-notes">All notes</h1>
        <div class="notes-grid__EMPTY_CLASS__" id="notes-grid">__NOTES__</div>
      </section>
    </main>
  </div>
  <script>
    const composer = document.querySelector(".composer");
    composer.addEventListener("focusin", () => composer.classList.add("is-open"));
    document.addEventListener("click", (event) => {
      if (!composer.contains(event.target)
          && !composer.elements.title.value.trim()
          && !composer.elements.content.value.trim()) {
        composer.classList.remove("is-open");
      }
    });
    const search = document.querySelector("#note-search");
    search.addEventListener("input", () => {
      const query = search.value.trim().toLocaleLowerCase();
      document.querySelectorAll(".note").forEach((note) => {
        note.hidden = !note.textContent.toLocaleLowerCase().includes(query);
      });
    });
  </script>
</body>
</html>"###;
    let empty_state = r#"<p class="empty"><svg class="empty-icon" viewBox="0 0 64 64" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M22 14h20a5 5 0 0 1 5 5v31H17V19a5 5 0 0 1 5-5Z"/><path d="M25 24h14m-14 8h14m-14 8h9"/></svg>Your notes will appear here</p>"#;
    let note_contents = if notes_empty {
        empty_state.to_owned()
    } else {
        notes_html
    };
    page.replace("__FORM__", &form)
        .replace(
            "__EMPTY_CLASS__",
            if empty_class.is_empty() {
                ""
            } else {
                " is-empty"
            },
        )
        .replace("__NOTES__", &note_contents)
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS notes (
            id BIGSERIAL PRIMARY KEY,
            title TEXT NOT NULL CHECK (length(trim(title)) > 0),
            content TEXT NOT NULL DEFAULT ''
        )",
    )
    .execute(&pool)
    .await?;

    let app = Router::new()
        .route("/", get(index))
        .route("/notes", post(create_note))
        .route("/notes/{id}/delete", post(delete_note))
        .with_state(pool);

    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_owned())
        .parse::<u16>()?;
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Notes app listening on {address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::{Note, escape_html, render_page};

    #[test]
    fn escapes_html_in_note_fields() {
        let notes = [Note {
            id: 1,
            title: "<script>alert('x')</script>".to_owned(),
            content: "A & B".to_owned(),
        }];

        let page = render_page(&notes);
        assert!(page.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"));
        assert!(page.contains("A &amp; B"));
        assert!(!page.contains("<script>alert('x')</script>"));
        assert!(page.contains("id=\"note-search\""));
        assert!(page.contains("class=\"note note-color-1\""));
        assert!(page.contains("Take a note..."));
    }

    #[test]
    fn escapes_all_html_special_characters() {
        assert_eq!(escape_html("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
    }
}
