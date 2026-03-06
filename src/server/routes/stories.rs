use axum::extract::State;
use axum::response::IntoResponse;

use crate::compute::stories::{derive_stories, top_stories};
use crate::server::AppState;
use crate::server::error::AppError;
use crate::server::templates::{HtmlTemplate, RegionStoryGroup, StoriesTemplate, story_to_view};

use super::load_story_inputs;

/// Stories page handler.
pub(super) async fn stories_page(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let inputs = load_story_inputs(&state.pool, None).await?;

    let mut all_stories = Vec::new();
    let mut national_stories = Vec::new();
    let mut region_groups = Vec::new();

    for input in &inputs {
        let stories = derive_stories(input);
        if input.region == "national" {
            national_stories = stories.iter().map(story_to_view).collect();
        } else if !stories.is_empty() {
            region_groups.push(RegionStoryGroup {
                slug: input.region.clone(),
                name: input.region_name.clone(),
                stories: stories.iter().map(story_to_view).collect(),
            });
        }
        all_stories.extend(stories);
    }

    let top = top_stories(&all_stories, 6);
    let top_views = top.iter().map(story_to_view).collect();

    let refreshing = state.refreshing.load(std::sync::atomic::Ordering::Relaxed);

    let tpl =
        StoriesTemplate { national_stories, top_stories: top_views, region_groups, refreshing };
    Ok(HtmlTemplate(tpl))
}
