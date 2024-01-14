use lazy_static::lazy_static;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/ssr_modes.css"/>
        <Title text="Welcome to Leptos"/>

        <nav style="display: flex; gap: 1em;">
            <a href="/">"/(home default/out-of-order)"</a>
            <a href="/home/in-order">"/home/in-order"</a>
            <a href="/home/async">"/home/async"</a>
            <a href="/home/partially-blocked">"/home/partially-blocked"</a>
        </nav>

        <p>"Disable javascript to see what async renders server-side vs. others."</p>

        <Router fallback>
            <main>
                <Routes>
                    // We’ll load the home page with out-of-order streaming and <Suspense/>
                    <Route path="" view=HomePage/>
                    <Route path="/home/in-order" view=HomePage ssr=SsrMode::InOrder/>
                    <Route path="/home/async" view=HomePage ssr=SsrMode::Async/>
                    <Route path="/home/partially-blocked" view=HomePage ssr=SsrMode::PartiallyBlocked/>

                    // We'll load the posts with async rendering, so they can set
                    // the title and metadata *after* loading the data
                    <Route
                        path="/post/:id"
                        view=PostPage
                        ssr=SsrMode::Async
                    />
                    <Route
                        path="/post_in_order/:id"
                        view=PostPage
                        ssr=SsrMode::InOrder
                    />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    // load the posts
    let posts =
        create_blocking_resource(|| (), |_| async { list_post_metadata().await });
    let posts_view = move || {
        posts.and_then(|posts| {
                        posts.iter()
                            .map(|post| view! {
                                <Post id=post.id />
                            })
                            .collect_view()
                    })
    };

    view! {
        <h1>"My Great Blog"</h1>
        <Suspense fallback=move || view! { <p>"Loading posts..."</p> }>
            {posts_view}
        </Suspense>
    }
}

#[derive(Params, Copy, Clone, Debug, PartialEq, Eq)]
pub struct PostParams {
    id: usize,
}

#[component]
fn PostPage() -> impl IntoView {
    let query = use_params::<PostParams>();
    let id = move || {
        query.with(|q| {
            q.as_ref().map(|q| q.id).map_err(|_| PostError::InvalidId)
        })
    };

    let view = move || {
        id().map(|id| view! {
            <Post id />
        })
    };
        

    view! {
        {view}
    }
}

#[component]
fn Post(id: usize) -> impl IntoView {
    let post = create_resource(|| (), move |_| async move {
        match id {
            id => get_post(id)
                .await
                .map(|data| data.ok_or(PostError::PostNotFound))
                .map_err(|_| PostError::ServerError)
                .flatten(),
        }
    });

    let post_view = move || {
        post.and_then(|post| {
            view! {
                // render content
                <h1>{&post.title}</h1>
                <p>{&post.content}</p>

                // since we're using async rendering for this page,
                // this metadata should be included in the actual HTML <head>
                // when it's first served
                // <Title text=post.title.clone()/>
                // <Meta name="description" content=post.content.clone()/>
            }
        })
    };

    view! {
        <Suspense fallback=move || view! { <p>"Loading post..."</p> }>
            <ErrorBoundary fallback=|errors| {
                view! {
                    <div class="error">
                        <h1>"Something went wrong."</h1>
                        <ul>
                        {move || errors.get()
                            .into_iter()
                            .map(|(_, error)| view! { <li>{error.to_string()} </li> })
                            .collect_view()
                        }
                        </ul>
                    </div>
                }
            }>
                <div class="post" style="border: 1px solid black; margin-bottom: 1em;">
                {post_view}
                <Comments post_id=id />
                </div>
            </ErrorBoundary>
        </Suspense>
    }
}

#[component]
fn Comments(post_id: usize) -> impl IntoView {
    let comments = create_resource(|| (), move |_| async move {
         get_comments(post_id)
            .await
            .map_err(|_| PostError::ServerError)
    });

    let view = move || comments.and_then(|comments| {
        view! {
            <div style="border: 1px dashed red;">
                <p>"Found " {comments.len()} " comments."</p>
            </div>
        }
    });

    view! {
        <Suspense>
            {view}
        </Suspense>
    }
}

// Dummy API
lazy_static! {
    static ref POSTS: Vec<Post> = vec![
        Post {
            id: 0,
            title: "My first post".to_string(),
            content: "This is my first post".to_string(),
        },
        Post {
            id: 1,
            title: "My second post".to_string(),
            content: "This is my second post".to_string(),
        },
        Post {
            id: 2,
            title: "My third post".to_string(),
            content: "This is my third post".to_string(),
        },
    ];
}

#[derive(Error, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostError {
    #[error("Invalid post ID.")]
    InvalidId,
    #[error("Post not found.")]
    PostNotFound,
    #[error("Server error.")]
    ServerError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    id: usize,
    title: String,
    content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostMetadata {
    id: usize,
    title: String,
}

#[server]
pub async fn list_post_metadata() -> Result<Vec<PostMetadata>, ServerFnError> {
    eprintln!("list_post_metadata: start");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    eprintln!("list_post_metadata: end");
    Ok(POSTS
        .iter()
        .map(|data| PostMetadata {
            id: data.id,
            title: data.title.clone(),
        })
        .collect())
}

#[server]
pub async fn get_post(id: usize) -> Result<Option<Post>, ServerFnError> {
    eprintln!("get_post: start");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    eprintln!("get_post: end");
    Ok(POSTS.iter().find(|post| post.id == id).cloned())
}



async fn get_comments(post_id: usize) -> Result<Vec<Comment>, ServerFnError> {
    eprintln!("get_comments: start");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    eprintln!("get_comments: end");
    Ok(vec![])
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Comment {
    // unused
}