use std::collections::{BTreeMap, HashSet};

use crate::mood::cache::CachedMoodSource;
use crate::mood::director::{MoodScores, NarrativeRole};
use crate::mood::inference::{extract_content, LlamaServer};
use crate::mood::winner::{self, ImageRole, PrepMode, RepairPlan};
use crate::types::{MoodCategory, MoodIntensity};

#[derive(Clone, Debug)]
pub struct PipelineActivity {
    pub phase: &'static str,
    pub page: u32,
    pub started_at_ms: u64,
}

#[derive(Clone)]
struct StoredPage {
    raw_bytes: Vec<u8>,
}

pub(crate) type WindowPrediction = winner::WindowPrediction;
type PageVoteStats = winner::VoteStats;
pub(crate) type PipelinePagePrediction = winner::AggregatedPrediction;
pub(crate) use winner::{RepairAcceptPair, RepairAcceptSingle, SelectivePrompt};

#[derive(Clone, Debug)]
pub(crate) struct ActionPage {
    pub(crate) page: u32,
    pub(crate) raw_bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub(crate) struct ActionTriplet {
    left: ActionPage,
    center: ActionPage,
    right: ActionPage,
}

#[derive(Clone, Debug)]
pub enum PipelineActionRequest {
    Window {
        center: u32,
        total_pages: u32,
        members: Vec<ActionPage>,
    },
    RepairSingle {
        key: String,
        page: u32,
        total_pages: u32,
        prompt_kind: SelectivePrompt,
        reason: &'static str,
        accept: RepairAcceptSingle,
        triplet: ActionTriplet,
    },
    RepairPair {
        key: String,
        first_page: u32,
        second_page: u32,
        total_pages: u32,
        prompt_kind: SelectivePrompt,
        reason: &'static str,
        accept: RepairAcceptPair,
        first_triplet: ActionTriplet,
        second_triplet: ActionTriplet,
    },
}

#[derive(Clone, Debug)]
pub struct PublishedPageUpdate {
    pub chapter: String,
    pub page: u32,
    pub mood: MoodCategory,
    pub intensity: MoodIntensity,
    pub scores: MoodScores,
    pub narrative_role: NarrativeRole,
    pub source: CachedMoodSource,
    pub finalized: bool,
}

pub struct ChapterMoodPipeline {
    current_chapter: Option<String>,
    total_pages: Option<u32>,
    focus_page: Option<u32>,
    focus_direction: i8,
    pages: BTreeMap<u32, StoredPage>,
    window_predictions: BTreeMap<u32, WindowPrediction>,
    base_predictions: BTreeMap<u32, PipelinePagePrediction>,
    effective_predictions: BTreeMap<u32, PipelinePagePrediction>,
    processing: bool,
    active_activity: Option<PipelineActivity>,
    last_error: Option<String>,
    pending_windows: HashSet<u32>,
    in_flight_repair_keys: HashSet<String>,
    completed_repair_keys: HashSet<String>,
}

impl ChapterMoodPipeline {
    pub fn new() -> Self {
        Self {
            current_chapter: None,
            total_pages: None,
            focus_page: None,
            focus_direction: 0,
            pages: BTreeMap::new(),
            window_predictions: BTreeMap::new(),
            base_predictions: BTreeMap::new(),
            effective_predictions: BTreeMap::new(),
            processing: false,
            active_activity: None,
            last_error: None,
            pending_windows: HashSet::new(),
            in_flight_repair_keys: HashSet::new(),
            completed_repair_keys: HashSet::new(),
        }
    }

    pub fn register_page(
        &mut self,
        chapter: &str,
        page: u32,
        total_pages: Option<u32>,
        raw_bytes: Vec<u8>,
    ) {
        if self.current_chapter.as_deref() != Some(chapter) {
            self.reset_for_chapter(chapter);
        }
        if let Some(total_pages) = total_pages {
            self.total_pages = Some(self.total_pages.unwrap_or(total_pages).max(total_pages));
        }
        self.pages.entry(page).or_insert(StoredPage { raw_bytes });
    }

    pub fn update_focus(
        &mut self,
        chapter: &str,
        page: u32,
        direction: i8,
        total_pages: Option<u32>,
    ) {
        if self.current_chapter.as_deref() != Some(chapter) {
            self.reset_for_chapter(chapter);
        }
        if let Some(total_pages) = total_pages {
            self.total_pages = Some(self.total_pages.unwrap_or(total_pages).max(total_pages));
        }
        self.focus_page = Some(page);
        self.focus_direction = direction.clamp(-1, 1);
    }

    pub fn registered_pages(&self) -> Vec<u32> {
        self.pages.keys().copied().collect()
    }

    pub fn focus_page(&self) -> Option<u32> {
        self.focus_page
    }

    pub fn is_processing(&self) -> bool {
        self.processing
    }

    pub fn active_activity(&self) -> Option<PipelineActivity> {
        self.active_activity.clone()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }

    pub fn start_processing_if_idle(&mut self) -> bool {
        if self.processing {
            return false;
        }
        self.processing = true;
        self.last_error = None;
        true
    }

    pub fn finish_processing(&mut self) {
        self.processing = false;
        self.active_activity = None;
    }

    pub fn next_action(&mut self) -> Option<PipelineActionRequest> {
        let total_pages = self.total_pages?;
        let request = self
            .next_window_action(total_pages)
            .or_else(|| self.next_repair_action(total_pages));
        if let Some(request) = request.as_ref() {
            self.active_activity = Some(PipelineActivity {
                phase: request.phase_name(),
                page: request.anchor_page(),
                started_at_ms: now_ms(),
            });
            self.last_error = None;
        } else {
            self.active_activity = None;
        }
        request
    }

    pub fn commit_success(
        &mut self,
        request: &PipelineActionRequest,
        result: PipelineActionResult,
    ) -> Vec<PublishedPageUpdate> {
        self.active_activity = None;
        self.last_error = None;
        match (request, result) {
            (
                PipelineActionRequest::Window { center, .. },
                PipelineActionResult::Window(prediction),
            ) => {
                self.pending_windows.remove(center);
                self.window_predictions.insert(*center, prediction);
                let mut updates = self.finalize_ready_pages();
                updates.extend(self.apply_structural_repairs());
                updates
            }
            (
                PipelineActionRequest::RepairSingle {
                    key, page, reason, ..
                },
                PipelineActionResult::RepairSingle(Some(prediction)),
            ) => {
                self.in_flight_repair_keys.remove(key);
                self.completed_repair_keys.insert(key.clone());
                self.apply_repair(*page, prediction, reason)
            }
            (
                PipelineActionRequest::RepairSingle { key, .. },
                PipelineActionResult::RepairSingle(None),
            ) => {
                self.in_flight_repair_keys.remove(key);
                self.completed_repair_keys.insert(key.clone());
                Vec::new()
            }
            (
                PipelineActionRequest::RepairPair {
                    key,
                    first_page,
                    second_page,
                    reason,
                    ..
                },
                PipelineActionResult::RepairPair(Some((first, second))),
            ) => {
                self.in_flight_repair_keys.remove(key);
                self.completed_repair_keys.insert(key.clone());
                let mut updates = self.apply_repair(*first_page, first, reason);
                updates.extend(self.apply_repair(*second_page, second, reason));
                updates
            }
            (
                PipelineActionRequest::RepairPair { key, .. },
                PipelineActionResult::RepairPair(None),
            ) => {
                self.in_flight_repair_keys.remove(key);
                self.completed_repair_keys.insert(key.clone());
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub fn commit_failure(&mut self, request: &PipelineActionRequest) {
        self.active_activity = None;
        match request {
            PipelineActionRequest::Window { center, .. } => {
                self.pending_windows.remove(center);
            }
            PipelineActionRequest::RepairSingle { key, .. }
            | PipelineActionRequest::RepairPair { key, .. } => {
                self.in_flight_repair_keys.remove(key);
            }
        }
    }

    fn reset_for_chapter(&mut self, chapter: &str) {
        self.current_chapter = Some(chapter.to_string());
        self.total_pages = None;
        self.focus_page = None;
        self.focus_direction = 0;
        self.pages.clear();
        self.window_predictions.clear();
        self.base_predictions.clear();
        self.effective_predictions.clear();
        self.pending_windows.clear();
        self.active_activity = None;
        self.last_error = None;
        self.in_flight_repair_keys.clear();
        self.completed_repair_keys.clear();
        self.processing = false;
    }

    pub fn record_error(&mut self, error: impl Into<String>) {
        self.active_activity = None;
        self.last_error = Some(error.into());
    }

    fn chapter_key(&self) -> Option<String> {
        self.current_chapter.clone()
    }

    fn action_page(&self, page: u32) -> Option<ActionPage> {
        let stored = self.pages.get(&page)?;
        Some(ActionPage {
            page,
            raw_bytes: stored.raw_bytes.clone(),
        })
    }

    fn window_ready(&self, center: u32, total_pages: u32) -> bool {
        winner::wide_context_indices(center, total_pages)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .all(|page| self.pages.contains_key(&page))
    }

    fn page_ready(&self, page: u32, total_pages: u32) -> bool {
        let start = page.saturating_sub(2);
        let end = (page + 2).min(total_pages.saturating_sub(1));
        (start..=end).all(|center| self.window_predictions.contains_key(&center))
    }

    fn finalized_page_order(&self) -> Vec<u32> {
        self.base_predictions.keys().copied().collect()
    }

    fn priority_key(&self, page: u32) -> (u32, u32, u32) {
        let Some(focus) = self.focus_page else {
            return (u32::MAX / 2, page, page);
        };
        if page == focus {
            return (0, 0, page);
        }

        let forward_bias = if self.focus_direction == 0 {
            1
        } else {
            self.focus_direction
        };

        if forward_bias >= 0 {
            if page > focus {
                let delta = page - focus;
                ((((delta - 1) / 2) * 2) + 1, delta, page)
            } else {
                let delta = focus - page;
                (delta * 2, delta, page)
            }
        } else if page < focus {
            let delta = focus - page;
            ((((delta - 1) / 2) * 2) + 1, delta, page)
        } else {
            let delta = page - focus;
            (delta * 2, delta, page)
        }
    }

    fn next_window_action(&mut self, total_pages: u32) -> Option<PipelineActionRequest> {
        let mut ready_centers = (0..total_pages)
            .filter(|center| {
                !self.window_predictions.contains_key(center)
                    && !self.pending_windows.contains(center)
                    && self.window_ready(*center, total_pages)
            })
            .collect::<Vec<_>>();
        ready_centers.sort_by_key(|center| self.priority_key(*center));

        let center = ready_centers.into_iter().next()?;
        let members = winner::wide_context_indices(center, total_pages)
            .into_iter()
            .map(|page| self.action_page(page))
            .collect::<Option<Vec<_>>>()?;
        self.pending_windows.insert(center);
        Some(PipelineActionRequest::Window {
            center,
            total_pages,
            members,
        })
    }

    fn finalize_ready_pages(&mut self) -> Vec<PublishedPageUpdate> {
        let mut updates = Vec::new();
        let Some(total_pages) = self.total_pages else {
            return updates;
        };
        let mut ready_pages = (0..total_pages)
            .filter(|page| {
                !self.base_predictions.contains_key(page) && self.page_ready(*page, total_pages)
            })
            .collect::<Vec<_>>();
        ready_pages.sort_by_key(|page| self.priority_key(*page));

        for page in ready_pages {
            let prediction = self.aggregate_page(page);
            self.base_predictions.insert(page, prediction.clone());
            self.effective_predictions.insert(page, prediction.clone());
            if let Some(update) = self.build_update(page, &prediction) {
                updates.push(update);
            }
        }
        updates
    }

    fn aggregate_page(&self, page: u32) -> PipelinePagePrediction {
        let stats = self.build_vote_stats(page);
        winner::aggregate_prediction(&stats)
    }

    fn build_vote_stats(&self, page: u32) -> PageVoteStats {
        let windows = self
            .window_predictions
            .values()
            .cloned()
            .collect::<Vec<_>>();
        winner::build_vote_stats(&windows, page)
    }

    fn next_repair_action(&mut self, total_pages: u32) -> Option<PipelineActionRequest> {
        let page_order = self.finalized_page_order();
        if page_order.is_empty() {
            return None;
        }

        let mut plans = Vec::new();
        for plan in winner::plan_selective_repairs(&page_order, &self.base_predictions) {
            let priority = match &plan {
                RepairPlan::Single { page, .. } => self.priority_key(*page),
                RepairPlan::Pair { first_page, .. } => self.priority_key(*first_page),
            };
            plans.push((priority, plan));
        }
        plans.sort_by_key(|(priority, _)| *priority);

        for (_, plan) in plans {
            match plan {
                RepairPlan::Single {
                    key,
                    page,
                    prompt_kind,
                    reason,
                    accept,
                } => {
                    if let Some(request) =
                        self.make_single_repair(total_pages, key, page, prompt_kind, reason, accept)
                    {
                        return Some(request);
                    }
                }
                RepairPlan::Pair {
                    key,
                    first_page,
                    second_page,
                    prompt_kind,
                    reason,
                    accept,
                } => {
                    if let Some(request) = self.make_pair_repair(
                        total_pages,
                        key,
                        first_page,
                        second_page,
                        prompt_kind,
                        reason,
                        accept,
                    ) {
                        return Some(request);
                    }
                }
            }
        }

        None
    }

    fn make_single_repair(
        &mut self,
        total_pages: u32,
        key: String,
        page: u32,
        prompt_kind: SelectivePrompt,
        reason: &'static str,
        accept: RepairAcceptSingle,
    ) -> Option<PipelineActionRequest> {
        if self.completed_repair_keys.contains(&key) || self.in_flight_repair_keys.contains(&key) {
            return None;
        }
        let triplet = self.make_triplet(page)?;
        self.in_flight_repair_keys.insert(key.clone());
        Some(PipelineActionRequest::RepairSingle {
            key,
            page,
            total_pages,
            prompt_kind,
            reason,
            accept,
            triplet,
        })
    }

    fn make_pair_repair(
        &mut self,
        total_pages: u32,
        key: String,
        first_page: u32,
        second_page: u32,
        prompt_kind: SelectivePrompt,
        reason: &'static str,
        accept: RepairAcceptPair,
    ) -> Option<PipelineActionRequest> {
        if self.completed_repair_keys.contains(&key) || self.in_flight_repair_keys.contains(&key) {
            return None;
        }
        let first_triplet = self.make_triplet(first_page)?;
        let second_triplet = self.make_triplet(second_page)?;
        self.in_flight_repair_keys.insert(key.clone());
        Some(PipelineActionRequest::RepairPair {
            key,
            first_page,
            second_page,
            total_pages,
            prompt_kind,
            reason,
            accept,
            first_triplet,
            second_triplet,
        })
    }

    fn make_triplet(&self, page: u32) -> Option<ActionTriplet> {
        let total_pages = self.total_pages?;
        let left = page.saturating_sub(1);
        let right = (page + 1).min(total_pages.saturating_sub(1));
        Some(ActionTriplet {
            left: self.action_page(left)?,
            center: self.action_page(page)?,
            right: self.action_page(right)?,
        })
    }

    fn apply_repair(
        &mut self,
        page: u32,
        mut prediction: PipelinePagePrediction,
        reason: &str,
    ) -> Vec<PublishedPageUpdate> {
        prediction.note = Some(reason.to_string());
        self.effective_predictions.insert(page, prediction.clone());
        self.build_update(page, &prediction).into_iter().collect()
    }

    fn build_update(
        &self,
        page: u32,
        prediction: &PipelinePagePrediction,
    ) -> Option<PublishedPageUpdate> {
        Some(PublishedPageUpdate {
            chapter: self.chapter_key()?,
            page,
            mood: prediction.mood,
            intensity: prediction.intensity,
            scores: prediction.scores.clone(),
            narrative_role: prediction.narrative_role,
            source: CachedMoodSource::ChapterPipeline,
            finalized: true,
        })
    }

    fn apply_structural_repairs(&mut self) -> Vec<PublishedPageUpdate> {
        let page_order = self.finalized_page_order();
        if page_order.is_empty() {
            return Vec::new();
        }
        let before = self.effective_predictions.clone();
        winner::apply_action_bridge_holds(
            &page_order,
            &self.base_predictions,
            &mut self.effective_predictions,
        );

        let mut updates = Vec::new();
        for page in page_order {
            let before_prediction = before.get(&page);
            let after_prediction = self.effective_predictions.get(&page);
            let unchanged = match (before_prediction, after_prediction) {
                (Some(before), Some(after)) => {
                    before.mood == after.mood
                        && before.intensity == after.intensity
                        && before.note == after.note
                }
                (None, None) => true,
                _ => false,
            };
            if unchanged {
                continue;
            }
            if let Some(prediction) = after_prediction {
                if let Some(update) = self.build_update(page, prediction) {
                    updates.push(update);
                }
            }
        }

        updates
    }
}

impl PipelineActionRequest {
    fn anchor_page(&self) -> u32 {
        match self {
            PipelineActionRequest::Window { center, .. } => *center,
            PipelineActionRequest::RepairSingle { page, .. } => *page,
            PipelineActionRequest::RepairPair { first_page, .. } => *first_page,
        }
    }

    fn phase_name(&self) -> &'static str {
        match self {
            PipelineActionRequest::Window { .. } => "window",
            PipelineActionRequest::RepairSingle { .. } => "repair_single",
            PipelineActionRequest::RepairPair { .. } => "repair_pair",
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone, Debug)]
pub enum PipelineActionResult {
    Window(WindowPrediction),
    RepairSingle(Option<PipelinePagePrediction>),
    RepairPair(Option<(PipelinePagePrediction, PipelinePagePrediction)>),
}

pub async fn execute_action(
    server: &mut LlamaServer,
    request: &PipelineActionRequest,
) -> Result<PipelineActionResult, String> {
    match request {
        PipelineActionRequest::Window {
            center,
            total_pages,
            members,
        } => {
            let context_indices = members.iter().map(|member| member.page).collect::<Vec<_>>();
            let prompt = winner::build_wide_sequence_prompt(&context_indices, *total_pages);
            let images = members
                .iter()
                .map(|page| encode_legacy_window_page(page))
                .collect::<Result<Vec<_>, _>>()?;
            let body = winner::build_request_body(
                &prompt,
                &images,
                winner::WINNER_BACKBONE_MAX_TOKENS,
                0.0,
                None,
                None,
                None,
                None,
            );
            let json = post_json_with_retry(server, &body).await?;
            let (mood, intensity) = winner::parse_prediction(&json)
                .map_err(|_| "Failed to parse wide-5 prediction".to_string())?;
            Ok(PipelineActionResult::Window(WindowPrediction {
                members: context_indices,
                center: *center,
                mood,
                intensity,
            }))
        }
        PipelineActionRequest::RepairSingle {
            total_pages,
            prompt_kind,
            triplet,
            accept,
            ..
        } => {
            let prediction = run_repair_prompt(server, *total_pages, *prompt_kind, triplet).await?;
            let accepted = winner::accepts_single(&prediction, *accept);
            Ok(PipelineActionResult::RepairSingle(
                accepted.then_some(prediction),
            ))
        }
        PipelineActionRequest::RepairPair {
            total_pages,
            prompt_kind,
            first_triplet,
            second_triplet,
            accept,
            ..
        } => {
            let first =
                run_repair_prompt(server, *total_pages, *prompt_kind, first_triplet).await?;
            let second =
                run_repair_prompt(server, *total_pages, *prompt_kind, second_triplet).await?;
            let accepted = winner::accepts_pair(&first, &second, *accept);
            Ok(PipelineActionResult::RepairPair(
                accepted.then_some((first, second)),
            ))
        }
    }
}

pub(crate) async fn analyze_visible_window(
    server: &mut LlamaServer,
    center: u32,
    total_pages: u32,
    members: Vec<ActionPage>,
) -> Result<PipelinePagePrediction, String> {
    let member_map = members
        .into_iter()
        .map(|member| (member.page, member))
        .collect::<BTreeMap<_, _>>();
    let page_order = member_map.keys().copied().collect::<Vec<_>>();

    let window_centers = page_order
        .iter()
        .copied()
        .filter(|candidate| {
            winner::wide_context_indices(*candidate, total_pages)
                .into_iter()
                .all(|page| member_map.contains_key(&page))
        })
        .collect::<Vec<_>>();

    if window_centers.is_empty() {
        return Err(
            "Visible winner window is incomplete. Need a wider local chapter neighborhood."
                .to_string(),
        );
    }

    let mut window_predictions = Vec::new();
    for window_center in window_centers {
        let context_pages = winner::wide_context_indices(window_center, total_pages);
        let prompt = winner::build_wide_sequence_prompt(&context_pages, total_pages);
        let images = context_pages
            .iter()
            .map(|page| {
                let action_page = member_map.get(page).ok_or_else(|| {
                    format!("Missing local page {} for visible winner window", page + 1)
                })?;
                encode_legacy_window_page(action_page)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let body = winner::build_request_body(
            &prompt,
            &images,
            winner::WINNER_BACKBONE_MAX_TOKENS,
            0.0,
            None,
            None,
            None,
            None,
        );
        let json = post_json_with_retry(server, &body).await?;
        let (mood, intensity) = winner::parse_prediction(&json).map_err(|_| {
            let raw = extract_content(&json).unwrap_or_else(|| "<no content>".to_string());
            let preview = raw.replace('\n', " ");
            format!(
                "Failed to parse visible-window prediction. Raw model output: {}",
                &preview[..preview.len().min(240)]
            )
        })?;
        window_predictions.push(WindowPrediction {
            members: context_pages,
            center: window_center,
            mood,
            intensity,
        });
    }

    let mut predictions = winner::aggregate_majority(&page_order, &window_predictions);
    for plan in winner::plan_selective_repairs(&page_order, &predictions) {
        match plan {
            RepairPlan::Single {
                page,
                prompt_kind,
                reason,
                accept,
                ..
            } => {
                if let Some(prediction) =
                    run_local_selective_repair(server, total_pages, &member_map, page, prompt_kind)
                        .await?
                {
                    if winner::accepts_single(&prediction, accept) {
                        let mut replacement = prediction;
                        replacement.note = Some(reason.to_string());
                        predictions.insert(page, replacement);
                    }
                }
            }
            RepairPlan::Pair {
                first_page,
                second_page,
                prompt_kind,
                reason,
                accept,
                ..
            } => {
                let first = run_local_selective_repair(
                    server,
                    total_pages,
                    &member_map,
                    first_page,
                    prompt_kind,
                )
                .await?;
                let second = run_local_selective_repair(
                    server,
                    total_pages,
                    &member_map,
                    second_page,
                    prompt_kind,
                )
                .await?;

                if let (Some(first), Some(second)) = (first, second) {
                    if winner::accepts_pair(&first, &second, accept) {
                        let mut first_replacement = first;
                        first_replacement.note = Some(reason.to_string());
                        predictions.insert(first_page, first_replacement);

                        let mut second_replacement = second;
                        second_replacement.note = Some(reason.to_string());
                        predictions.insert(second_page, second_replacement);
                    }
                }
            }
        }
    }

    predictions.remove(&center).ok_or_else(|| {
        format!(
            "Visible page {} not produced by winner aggregation",
            center + 1
        )
    })
}

fn encode_legacy_window_page(page: &ActionPage) -> Result<winner::EncodedImage, String> {
    winner::encode_image(&page.raw_bytes, PrepMode::Legacy672Jpeg, ImageRole::Context)
        .map_err(|e| format!("Failed to prepare page {}: {}", page.page, e))
}

fn encode_repair_triplet(triplet: &ActionTriplet) -> Result<Vec<winner::EncodedImage>, String> {
    Ok(vec![
        winner::encode_image(
            &triplet.left.raw_bytes,
            PrepMode::CenterPng1024,
            ImageRole::Context,
        )
        .map_err(|e| format!("Failed to prepare left page {}: {}", triplet.left.page, e))?,
        winner::encode_image(
            &triplet.center.raw_bytes,
            PrepMode::CenterPng1024,
            ImageRole::Center,
        )
        .map_err(|e| {
            format!(
                "Failed to prepare center page {}: {}",
                triplet.center.page, e
            )
        })?,
        winner::encode_image(
            &triplet.right.raw_bytes,
            PrepMode::CenterPng1024,
            ImageRole::Context,
        )
        .map_err(|e| format!("Failed to prepare right page {}: {}", triplet.right.page, e))?,
    ])
}

async fn run_repair_prompt(
    server: &mut LlamaServer,
    total_pages: u32,
    prompt_kind: SelectivePrompt,
    triplet: &ActionTriplet,
) -> Result<PipelinePagePrediction, String> {
    let prompt = match prompt_kind {
        SelectivePrompt::Focus => winner::build_focus_prompt(triplet.center.page, total_pages),
        SelectivePrompt::Narrative => {
            winner::build_narrative_focus_prompt(triplet.center.page, total_pages)
        }
    };
    let images = encode_repair_triplet(triplet)?;
    let body = winner::build_request_body(
        &prompt,
        &images,
        winner::WINNER_SELECTIVE_MAX_TOKENS,
        0.0,
        None,
        None,
        None,
        None,
    );
    let json = post_json_with_retry(server, &body).await?;
    let (mood, intensity) = winner::parse_prediction(&json)
        .map_err(|_| "Failed to parse repair prediction".to_string())?;
    Ok(PipelinePagePrediction {
        mood,
        intensity,
        winner_score: 0.0,
        runner_up_score: 0.0,
        scores: MoodScores::from_single(mood),
        narrative_role: winner::infer_narrative_role(mood, intensity),
        note: Some(prompt_kind.note().to_string()),
    })
}

async fn post_json_with_retry(
    server: &mut LlamaServer,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let url = format!("http://127.0.0.1:{}/v1/chat/completions", server.port);

    for attempt in 1..=3u32 {
        if let Ok(resp) = client.post(&url).json(body).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    return Ok(json);
                }
            }
        }
        if attempt < 3 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    Err("llama-server request failed after retries".to_string())
}

async fn run_local_selective_repair(
    server: &mut LlamaServer,
    total_pages: u32,
    member_map: &BTreeMap<u32, ActionPage>,
    page: u32,
    prompt_kind: SelectivePrompt,
) -> Result<Option<PipelinePagePrediction>, String> {
    let left_page = page.saturating_sub(1);
    let right_page = (page + 1).min(total_pages.saturating_sub(1));
    let (Some(left), Some(center), Some(right)) = (
        member_map.get(&left_page),
        member_map.get(&page),
        member_map.get(&right_page),
    ) else {
        return Ok(None);
    };

    let triplet = ActionTriplet {
        left: left.clone(),
        center: center.clone(),
        right: right.clone(),
    };

    run_repair_prompt(server, total_pages, prompt_kind, &triplet)
        .await
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BaseMood;

    #[test]
    fn winner_wide_context_indices_clamp_on_edges() {
        assert_eq!(winner::wide_context_indices(0, 10), vec![0, 0, 0, 1, 2]);
        assert_eq!(winner::wide_context_indices(9, 10), vec![7, 8, 9, 9, 9]);
    }

    #[test]
    fn structural_bridge_hold_promotes_first_two_tension_pages() {
        let base_prediction = |mood| PipelinePagePrediction {
            mood,
            intensity: MoodIntensity::High,
            winner_score: 0.0,
            runner_up_score: 0.0,
            scores: MoodScores::from_single(mood),
            narrative_role: winner::infer_narrative_role(mood, MoodIntensity::High),
            note: None,
        };

        let mut pipeline = ChapterMoodPipeline::new();
        pipeline.current_chapter = Some("BL/1".to_string());
        for (page, mood) in [
            BaseMood::Epic,
            BaseMood::Epic,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Epic,
            BaseMood::Epic,
            BaseMood::Epic,
            BaseMood::Epic,
        ]
        .into_iter()
        .enumerate()
        {
            let prediction = base_prediction(mood);
            pipeline
                .base_predictions
                .insert(page as u32, prediction.clone());
            pipeline
                .effective_predictions
                .insert(page as u32, prediction);
        }

        let updates = pipeline.apply_structural_repairs();

        assert_eq!(updates.len(), 2);
        assert_eq!(
            pipeline.effective_predictions.get(&2).map(|p| p.mood),
            Some(BaseMood::Epic)
        );
        assert_eq!(
            pipeline.effective_predictions.get(&3).map(|p| p.mood),
            Some(BaseMood::Epic)
        );
        assert_eq!(
            pipeline
                .effective_predictions
                .get(&2)
                .and_then(|p| p.note.as_deref()),
            Some("action_bridge_hold_epic")
        );
    }

    #[test]
    fn focus_priority_prefers_forward_hot_zone() {
        let mut pipeline = ChapterMoodPipeline::new();
        pipeline.update_focus("BL/1", 6, 1, Some(30));

        assert!(pipeline.priority_key(7) < pipeline.priority_key(5));
        assert!(pipeline.priority_key(8) < pipeline.priority_key(5));
        assert!(pipeline.priority_key(5) < pipeline.priority_key(9));
        assert!(pipeline.priority_key(9) < pipeline.priority_key(4));
    }

    #[test]
    fn next_action_prefers_ready_focus_window_over_chapter_start() {
        let mut pipeline = ChapterMoodPipeline::new();
        pipeline.update_focus("BL/1", 6, 1, Some(20));

        for page in 4..=8 {
            pipeline.register_page("BL/1", page, Some(20), vec![page as u8]);
        }

        let request = pipeline.next_action();
        match request {
            Some(PipelineActionRequest::Window { center, .. }) => assert_eq!(center, 6),
            other => panic!("expected focus window request, got {:?}", other),
        }
    }

    #[test]
    fn finalize_ready_pages_can_publish_hot_zone_without_earlier_pages() {
        let mut pipeline = ChapterMoodPipeline::new();
        pipeline.update_focus("BL/1", 6, 1, Some(20));
        pipeline.total_pages = Some(20);

        for center in 4..=8 {
            pipeline.window_predictions.insert(
                center,
                WindowPrediction {
                    members: winner::wide_context_indices(center, 20),
                    center,
                    mood: BaseMood::Tension,
                    intensity: MoodIntensity::Medium,
                },
            );
        }

        let updates = pipeline.finalize_ready_pages();

        assert!(updates.iter().any(|update| update.page == 6));
        assert!(pipeline.base_predictions.contains_key(&6));
        assert!(!pipeline.base_predictions.contains_key(&0));
    }
}
