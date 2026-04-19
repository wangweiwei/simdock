use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

mod i18n;

use i18n::{AppLanguage, ThemeMode, WindowTitleState};
use iced::{
    Alignment, Color, Element, Font, Length, Subscription, Task, Theme, application, border,
    futures::SinkExt,
    stream,
    theme::Palette,
    time,
    widget::{
        button, column, container, pick_list, row, scrollable,
        scrollable::{Direction, Scrollbar},
    },
};
use simdock_core::{
    DoctorReport, InstallRequest, Platform, TaskEvent,
    provider::{PlatformProvider, android::AndroidProvider, ios::IosProvider},
    service::SimdockService,
};
use simdock_infra::AppPaths;

const UI_FONT: Font = Font::with_name("PingFang SC");
const SYSTEM_CJK_FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/STHeiti Medium.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
];
const LANGUAGE_OPTIONS: [AppLanguage; 2] = [AppLanguage::Chinese, AppLanguage::English];
const STEPPER_DOT_SIZE: u16 = 6;
const STEPPER_LINE_HEIGHT: u16 = 1;
const STEPPER_NODE_WIDTH: u16 = 112;
const STEPPER_EDGE_INSET: u16 = 0;
const STEPPER_CONTENT_SPACING: u16 = 14;
const STEPPER_PADDING_VERTICAL: u16 = 22;
const STEPPER_PADDING_HORIZONTAL: u16 = 10;

pub fn main() -> iced::Result {
    prepare_system_fonts();

    let mut app = application(window_title, update, view);

    if let Some(font_bytes) = load_system_cjk_font() {
        app = app.font(font_bytes).default_font(UI_FONT);
    }

    app.theme(app_theme)
        .style(app_style)
        .subscription(subscription)
        .window_size((1100.0, 760.0))
        .centered()
        .run_with(|| (DoctorApp::loading(), load_doctor_task()))
}

fn prepare_system_fonts() {
    let Ok(mut font_system) = iced_graphics::text::font_system().write() else {
        return;
    };

    let raw = font_system.raw();
    raw.db_mut().load_system_fonts();
    raw.db_mut().set_sans_serif_family("PingFang SC");
}

#[derive(Debug, Clone)]
struct DoctorApp {
    status: LoadState,
    snapshot: Option<DoctorSnapshot>,
    active_tab: Platform,
    language: AppLanguage,
    theme_mode: ThemeMode,
    installs: InstallTasks,
    xcode_license_action: XcodeLicenseAction,
    spinner_frame: usize,
}

#[derive(Debug, Clone)]
struct DoctorSnapshot {
    reports: Vec<DoctorReport>,
}

#[derive(Debug, Clone)]
enum LoadState {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Debug, Clone)]
enum XcodeLicenseAction {
    Idle,
    Running,
}

#[derive(Debug, Clone)]
enum XcodeLicenseOutcome {
    Accepted,
    Cancelled,
}

#[derive(Debug, Clone)]
struct InstallTasks {
    ios: InstallTaskView,
    android: InstallTaskView,
}

#[derive(Debug, Clone)]
struct InstallTaskView {
    state: InstallState,
    progress: f32,
    current_step: String,
    logs: Vec<String>,
}

#[derive(Debug, Clone)]
enum InstallState {
    Idle,
    Running,
    Opening,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepVisualState {
    Done,
    Active,
    Blocked,
    Pending,
}

#[derive(Debug, Clone)]
enum InstallStreamEvent {
    Event(TaskEvent),
    Finished(Result<(), String>),
}

#[derive(Debug, Clone)]
enum Message {
    RefreshRequested,
    TabSelected(Platform),
    LanguageSelected(AppLanguage),
    ThemeSelected(ThemeMode),
    InstallRequested(Platform),
    InstallEvent(Platform, InstallStreamEvent),
    OpenSimulatorRequested(Platform),
    OpenSimulatorFinished(Platform, Result<(), String>),
    XcodeLicenseInstallAcceptFinished(Platform, Result<XcodeLicenseOutcome, String>),
    SpinnerTick,
    DoctorLoaded(Result<DoctorSnapshot, String>),
}

impl DoctorApp {
    fn loading() -> Self {
        Self {
            status: LoadState::Loading,
            snapshot: None,
            active_tab: Platform::Ios,
            language: AppLanguage::Chinese,
            theme_mode: ThemeMode::System,
            installs: InstallTasks::default(),
            xcode_license_action: XcodeLicenseAction::Idle,
            spinner_frame: 0,
        }
    }

    fn is_loading(&self) -> bool {
        matches!(self.status, LoadState::Loading)
    }

    fn install_task(&self, platform: Platform) -> &InstallTaskView {
        match platform {
            Platform::Ios => &self.installs.ios,
            Platform::Android => &self.installs.android,
        }
    }

    fn install_task_mut(&mut self, platform: Platform) -> &mut InstallTaskView {
        match platform {
            Platform::Ios => &mut self.installs.ios,
            Platform::Android => &mut self.installs.android,
        }
    }

    fn is_dark(&self) -> bool {
        match self.theme_mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => Theme::default() == Theme::Dark,
        }
    }

    fn has_running_work(&self) -> bool {
        self.installs.ios.is_busy()
            || self.installs.android.is_busy()
            || self.xcode_license_action.is_running()
    }
}

impl XcodeLicenseAction {
    fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

impl Default for InstallTasks {
    fn default() -> Self {
        Self {
            ios: InstallTaskView::idle(Platform::Ios),
            android: InstallTaskView::idle(Platform::Android),
        }
    }
}

impl InstallTaskView {
    fn idle(platform: Platform) -> Self {
        Self {
            state: InstallState::Idle,
            progress: 0.0,
            current_step: i18n::install_ready_to_start(platform, AppLanguage::Chinese),
            logs: Vec::new(),
        }
    }

    fn is_running(&self) -> bool {
        matches!(self.state, InstallState::Running)
    }

    fn is_opening(&self) -> bool {
        matches!(self.state, InstallState::Opening)
    }

    fn is_busy(&self) -> bool {
        self.is_running() || self.is_opening()
    }

    fn start(&mut self, platform: Platform, language: AppLanguage) {
        self.state = InstallState::Running;
        self.progress = 0.0;
        self.current_step = i18n::install_starting_message(platform, language);
        self.logs.clear();
    }

    fn start_open(&mut self, platform: Platform, language: AppLanguage) {
        self.state = InstallState::Opening;
        self.progress = 100.0;
        self.current_step = i18n::opening_simulator_message(platform, language);
        self.logs.clear();
        self.push_log(self.current_step.clone());
    }

    fn finish(&mut self, result: Result<(), String>, language: AppLanguage) {
        match result {
            Ok(()) => {
                self.state = InstallState::Completed;
                self.progress = 100.0;
                if self.current_step.is_empty() {
                    self.current_step = i18n::install_completed_message(language).to_string();
                }
            }
            Err(error) => {
                let error = i18n::install_message(&error, language);
                self.state = InstallState::Failed(error.clone());
                self.current_step = error;
            }
        }
    }

    fn finish_open(&mut self, result: Result<(), String>, language: AppLanguage) {
        match result {
            Ok(()) => {
                self.state = InstallState::Completed;
                self.progress = 100.0;
                self.current_step = i18n::simulator_opened_message(language).to_string();
                self.push_log(self.current_step.clone());
            }
            Err(error) => {
                let error = i18n::install_message(&error, language);
                self.state = InstallState::Failed(error.clone());
                self.current_step = error.clone();
                self.push_log(error);
            }
        }
    }

    fn wait_for_xcode_license(&mut self, language: AppLanguage) {
        self.state = InstallState::Running;
        self.current_step = i18n::xcode_license_waiting_message(language).to_string();
        self.push_log(self.current_step.clone());
    }

    fn apply_event(&mut self, event: TaskEvent, language: AppLanguage) {
        match event {
            TaskEvent::Started { title, .. } => {
                self.state = InstallState::Running;
                self.current_step = i18n::install_message(&title, language);
                self.push_log(self.current_step.clone());
            }
            TaskEvent::Progress { pct, message, .. } => {
                self.progress = pct.clamp(0.0, 100.0);
                self.current_step = i18n::install_message(&message, language);
                self.push_log(self.current_step.clone());
            }
            TaskEvent::Log { message, .. } => {
                self.push_log(i18n::install_message(&message, language));
            }
            TaskEvent::Finished { .. } => {
                self.state = InstallState::Completed;
                self.progress = 100.0;
                self.current_step = i18n::install_completed_message(language).to_string();
                self.push_log(self.current_step.clone());
            }
            TaskEvent::Failed { error, .. } => {
                let error = i18n::install_message(&error, language);
                self.state = InstallState::Failed(error.clone());
                self.current_step = error;
                self.push_log(self.current_step.clone());
            }
        }
    }

    fn push_log(&mut self, message: String) {
        self.logs.push(message);
        if self.logs.len() > 40 {
            self.logs.remove(0);
        }
    }
}

fn text<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    iced::widget::text(content).font(UI_FONT)
}

fn primary_text_color(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xEC, 0xF1, 0xF5)
    } else {
        Color::from_rgb8(0x1D, 0x2A, 0x3A)
    }
}

fn muted_text_color(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xA9, 0xB5, 0xC2)
    } else {
        Color::from_rgb8(0x5A, 0x67, 0x77)
    }
}

fn detail_text_color(dark: bool) -> Color {
    if dark {
        Color::from_rgb8(0xC2, 0xCB, 0xD5)
    } else {
        Color::from_rgb8(0x4F, 0x5E, 0x6E)
    }
}

fn window_title(app: &DoctorApp) -> String {
    let state = match &app.status {
        LoadState::Loading => WindowTitleState::Loading,
        LoadState::Ready => WindowTitleState::Ready,
        LoadState::Failed(_) => WindowTitleState::Failed,
    };

    i18n::window_title(app.language, app.active_tab, state)
}

fn app_theme(app: &DoctorApp) -> Theme {
    match app.theme_mode {
        ThemeMode::Light => simdock_theme(false),
        ThemeMode::Dark => simdock_theme(true),
        ThemeMode::System => simdock_theme(Theme::default() == Theme::Dark),
    }
}

fn simdock_theme(dark: bool) -> Theme {
    if dark {
        Theme::custom(
            "Simdock Night".to_string(),
            Palette {
                background: Color::from_rgb8(0x16, 0x1B, 0x22),
                text: Color::from_rgb8(0xEC, 0xF1, 0xF5),
                primary: Color::from_rgb8(0x2D, 0xB7, 0xBD),
                success: Color::from_rgb8(0x4D, 0xC1, 0x72),
                danger: Color::from_rgb8(0xE0, 0x6A, 0x57),
            },
        )
    } else {
        Theme::custom(
            "Simdock Day".to_string(),
            Palette {
                background: Color::from_rgb8(0xF6, 0xF1, 0xE8),
                text: Color::from_rgb8(0x20, 0x2A, 0x35),
                primary: Color::from_rgb8(0x0D, 0x74, 0x7A),
                success: Color::from_rgb8(0x2F, 0x7A, 0x44),
                danger: Color::from_rgb8(0xB2, 0x47, 0x38),
            },
        )
    }
}

fn app_style(_app: &DoctorApp, theme: &Theme) -> application::Appearance {
    let palette = theme.extended_palette();

    application::Appearance {
        background_color: palette.background.base.color,
        text_color: palette.background.base.text,
    }
}

fn load_system_cjk_font() -> Option<&'static [u8]> {
    find_system_font_file("PingFang.ttc")
        .and_then(|path| std::fs::read(path).ok())
        .or_else(|| {
            SYSTEM_CJK_FONT_PATHS
                .iter()
                .find_map(|path| std::fs::read(path).ok())
        })
        .map(|bytes| bytes.leak() as &'static [u8])
}

fn find_system_font_file(file_name: &str) -> Option<std::path::PathBuf> {
    let mut pending = vec![std::path::PathBuf::from(
        "/System/Library/AssetsV2/com_apple_MobileAsset_Font8",
    )];

    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(dir).ok()?;

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
                return Some(path);
            }
        }
    }

    None
}

fn update(app: &mut DoctorApp, message: Message) -> Task<Message> {
    match message {
        Message::SpinnerTick => {
            app.spinner_frame = app.spinner_frame.wrapping_add(1);
            Task::none()
        }
        Message::RefreshRequested => {
            app.status = LoadState::Loading;
            load_doctor_task()
        }
        Message::TabSelected(platform) => {
            app.active_tab = platform;
            Task::none()
        }
        Message::LanguageSelected(language) => {
            app.language = language;
            Task::none()
        }
        Message::ThemeSelected(theme_mode) => {
            app.theme_mode = theme_mode;
            Task::none()
        }
        Message::InstallRequested(platform) => {
            let language = app.language;
            app.install_task_mut(platform).start(platform, language);
            Task::run(run_install_stream(platform), move |event| {
                Message::InstallEvent(platform, event)
            })
        }
        Message::OpenSimulatorRequested(platform) => {
            let language = app.language;
            app.install_task_mut(platform)
                .start_open(platform, language);
            Task::perform(open_simulator(platform), move |result| {
                Message::OpenSimulatorFinished(platform, result)
            })
        }
        Message::OpenSimulatorFinished(platform, result) => {
            let language = app.language;
            app.install_task_mut(platform).finish_open(result, language);
            Task::none()
        }
        Message::InstallEvent(platform, event) => {
            let language = app.language;

            match event {
                InstallStreamEvent::Event(event) => {
                    app.install_task_mut(platform).apply_event(event, language);
                    Task::none()
                }
                InstallStreamEvent::Finished(result) => {
                    if platform == Platform::Ios
                        && result
                            .as_ref()
                            .err()
                            .is_some_and(|error| is_xcode_license_error(error))
                    {
                        app.install_task_mut(platform)
                            .wait_for_xcode_license(language);
                        app.xcode_license_action = XcodeLicenseAction::Running;
                        return Task::perform(accept_xcode_license(language), move |result| {
                            Message::XcodeLicenseInstallAcceptFinished(platform, result)
                        });
                    }

                    app.install_task_mut(platform).finish(result, language);
                    Task::none()
                }
            }
        }
        Message::XcodeLicenseInstallAcceptFinished(platform, result) => {
            let language = app.language;
            app.xcode_license_action = XcodeLicenseAction::Idle;

            match result {
                Ok(XcodeLicenseOutcome::Accepted) => {
                    app.install_task_mut(platform).start(platform, language);
                    Task::run(run_install_stream(platform), move |event| {
                        Message::InstallEvent(platform, event)
                    })
                }
                Ok(XcodeLicenseOutcome::Cancelled) => {
                    app.install_task_mut(platform).finish(
                        Err(i18n::xcode_license_cancelled_message(language).to_string()),
                        language,
                    );
                    Task::none()
                }
                Err(error) => {
                    app.install_task_mut(platform).finish(
                        Err(i18n::xcode_license_command_failed(&error, language)),
                        language,
                    );
                    Task::none()
                }
            }
        }
        Message::DoctorLoaded(result) => {
            match result {
                Ok(snapshot) => {
                    app.snapshot = Some(snapshot);
                    app.status = LoadState::Ready;
                }
                Err(error) => {
                    app.status = LoadState::Failed(error);
                }
            }

            Task::none()
        }
    }
}

fn subscription(app: &DoctorApp) -> Subscription<Message> {
    if app.has_running_work() {
        time::every(Duration::from_millis(140)).map(|_| Message::SpinnerTick)
    } else {
        Subscription::none()
    }
}

fn view(app: &DoctorApp) -> Element<'_, Message> {
    let dark = app.is_dark();
    let active_report = app
        .snapshot
        .as_ref()
        .and_then(|snapshot| report_for_platform(snapshot, app.active_tab));

    let header = row![
        column![
            text(i18n::app_title())
                .size(40)
                .color(primary_text_color(dark)),
            text(i18n::header_subtitle(app.language))
                .size(16)
                .color(muted_text_color(dark)),
        ]
        .spacing(8)
        .width(Length::Fill),
        top_right_controls(app, dark),
    ]
    .spacing(24)
    .align_y(Alignment::Center);

    let status_banner = status_banner(app, active_report);
    let tab_bar = platform_tabs(app.active_tab);
    let install_panel = install_panel(
        app.active_tab,
        app.install_task(app.active_tab),
        active_report,
        app.language,
        dark,
        app.spinner_frame,
    );

    let content = column![header, status_banner, tab_bar, install_panel].spacing(20);

    scrollable(
        container(content.padding(28).spacing(24))
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
    .direction(Direction::Vertical(
        Scrollbar::new().width(18).scroller_width(8).margin(0),
    ))
    .style(main_scrollbar_style)
    .into()
}

fn top_right_controls(app: &DoctorApp, dark: bool) -> Element<'_, Message> {
    let theme_options = i18n::theme_mode_options(app.language);

    row![
        column![
            text(i18n::language_field_label(app.language))
                .size(12)
                .color(muted_text_color(dark)),
            pick_list(
                LANGUAGE_OPTIONS,
                Some(app.language),
                Message::LanguageSelected
            )
            .font(UI_FONT)
            .text_size(14)
            .padding([9, 12])
            .width(132),
        ]
        .spacing(6),
        column![
            text(i18n::theme_field_label(app.language))
                .size(12)
                .color(muted_text_color(dark)),
            pick_list(
                theme_options,
                Some(i18n::theme_mode_option(app.theme_mode, app.language)),
                |option| Message::ThemeSelected(option.mode),
            )
            .font(UI_FONT)
            .text_size(14)
            .padding([9, 12])
            .width(152),
        ]
        .spacing(6),
    ]
    .spacing(12)
    .align_y(Alignment::End)
    .into()
}

fn refresh_button(is_loading: bool, language: AppLanguage) -> Element<'static, Message> {
    let label = i18n::refresh_button_label(is_loading, language);

    let mut button = button(text(label).size(15))
        .padding([10, 16])
        .style(secondary_button_style);

    if !is_loading {
        button = button.on_press(Message::RefreshRequested);
    }

    button.into()
}

fn status_banner(
    app: &DoctorApp,
    active_report: Option<&DoctorReport>,
) -> Element<'static, Message> {
    let (title, detail, ready) = match &app.status {
        LoadState::Loading => (
            i18n::status_loading_title(app.language).to_string(),
            i18n::status_loading_detail(app.language).to_string(),
            true,
        ),
        LoadState::Ready => match active_report {
            Some(report) if report.ready => (
                i18n::selected_ready_title(report.platform, app.language).to_string(),
                i18n::selected_ready_detail(app.language).to_string(),
                true,
            ),
            Some(report) => (
                i18n::selected_attention_title(report.platform, app.language).to_string(),
                i18n::selected_attention_detail(report.platform, app.language).to_string(),
                false,
            ),
            None => (
                i18n::no_diagnostics_title(app.language).to_string(),
                i18n::no_diagnostics_detail(app.language).to_string(),
                false,
            ),
        },
        LoadState::Failed(error) => (
            i18n::doctor_failed_title(app.language).to_string(),
            error.clone(),
            false,
        ),
    };

    container(
        row![
            column![text(title).size(24), text(detail).size(15),]
                .spacing(8)
                .width(Length::Fill),
            refresh_button(app.is_loading(), app.language),
        ]
        .spacing(18)
        .align_y(Alignment::Center),
    )
    .padding(22)
    .width(Length::Fill)
    .style(move |theme| banner_style(theme, ready))
    .into()
}

fn platform_tabs(active_tab: Platform) -> Element<'static, Message> {
    row![
        tab_button(Platform::Ios, active_tab),
        tab_button(Platform::Android, active_tab),
    ]
    .spacing(12)
    .into()
}

fn tab_button(platform: Platform, active_tab: Platform) -> Element<'static, Message> {
    let label = i18n::platform_label(platform);

    if platform == active_tab {
        container(text(label).size(16).color(Color::WHITE))
            .padding([12, 18])
            .style(move |theme| pill_style(theme, Color::from_rgb8(0x0D, 0x74, 0x7A)))
            .into()
    } else {
        button(text(label).size(16))
            .padding([12, 18])
            .style(inactive_tab_button_style)
            .on_press(Message::TabSelected(platform))
            .into()
    }
}

fn install_panel<'a>(
    platform: Platform,
    task: &'a InstallTaskView,
    report: Option<&'a DoctorReport>,
    language: AppLanguage,
    dark: bool,
    spinner_frame: usize,
) -> Element<'a, Message> {
    let simulator_ready =
        matches!(task.state, InstallState::Completed) || report.is_some_and(|report| report.ready);
    let action_label = if task.is_opening() {
        i18n::action_opening_label(language)
    } else if task.is_running() {
        i18n::action_installing_label(language)
    } else if simulator_ready {
        i18n::action_open_simulator_label(language)
    } else {
        i18n::action_one_click_install_label(language)
    };

    let mut action = button(text(action_label).size(15))
        .padding([12, 20])
        .style(primary_button_style);
    if !task.is_busy() {
        action = if simulator_ready {
            action.on_press(Message::OpenSimulatorRequested(platform))
        } else {
            action.on_press(Message::InstallRequested(platform))
        };
    }

    let body = column![
        row![
            column![
                text(i18n::install_panel_title(platform, language)).size(22),
                text(i18n::install_hint(platform, language))
                    .size(15)
                    .color(muted_text_color(dark)),
            ]
            .spacing(6)
            .width(Length::Fill),
            action,
        ]
        .spacing(16)
        .align_y(Alignment::Center),
        install_stage_stepper(platform, task, report, language, dark, spinner_frame),
        install_log_panel(task, language, dark),
    ]
    .spacing(18);

    container(body)
        .padding(22)
        .width(Length::Fill)
        .style(section_card)
        .into()
}

fn install_stage_stepper(
    platform: Platform,
    task: &InstallTaskView,
    report: Option<&DoctorReport>,
    language: AppLanguage,
    dark: bool,
    spinner_frame: usize,
) -> Element<'static, Message> {
    let titles = i18n::install_stage_titles(platform, language);
    let states = install_stage_states(platform, task, report);
    let mut track = row![stepper_edge_spacer()]
        .spacing(0)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let mut labels = row![stepper_edge_spacer()]
        .spacing(0)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    for index in 0..titles.len() {
        let left_connector = if index == 0 {
            None
        } else {
            Some(stage_connector_state(states[index - 1]))
        };
        let right_connector = if index + 1 == titles.len() {
            None
        } else {
            Some(stage_connector_state(states[index]))
        };

        track = track.push(install_stage_node(
            states[index],
            left_connector,
            right_connector,
            spinner_frame,
        ));
        labels = labels.push(install_stage_label_node(titles[index], states[index], dark));

        if index + 1 < titles.len() {
            let connector_state = stage_connector_state(states[index]);
            track = track.push(install_stage_segment(connector_state));
            labels = labels.push(install_stage_gap());
        }
    }

    track = track.push(stepper_edge_spacer());
    labels = labels.push(stepper_edge_spacer());

    container(column![track, labels].spacing(STEPPER_CONTENT_SPACING))
        .padding([STEPPER_PADDING_VERTICAL, STEPPER_PADDING_HORIZONTAL])
        .width(Length::Fill)
        .style(stepper_card_style)
        .into()
}

fn stage_connector_state(state: StepVisualState) -> StepVisualState {
    if matches!(state, StepVisualState::Done | StepVisualState::Active) {
        state
    } else {
        StepVisualState::Pending
    }
}

fn install_stage_node(
    state: StepVisualState,
    left_connector: Option<StepVisualState>,
    right_connector: Option<StepVisualState>,
    spinner_frame: usize,
) -> Element<'static, Message> {
    container(
        row![
            install_stage_half_segment(left_connector),
            install_stage_dot(state, spinner_frame),
            install_stage_half_segment(right_connector),
        ]
        .spacing(0)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .width(STEPPER_NODE_WIDTH)
    .into()
}

fn install_stage_dot(state: StepVisualState, spinner_frame: usize) -> Element<'static, Message> {
    container(text(""))
        .width(STEPPER_DOT_SIZE)
        .height(STEPPER_DOT_SIZE)
        .style(move |theme| step_dot_style(theme, state, spinner_frame))
        .into()
}

fn install_stage_label_node(
    title: &'static str,
    state: StepVisualState,
    dark: bool,
) -> Element<'static, Message> {
    container(text(title).size(18).color(step_label_color(state, dark)))
        .width(STEPPER_NODE_WIDTH)
        .align_x(iced::alignment::Horizontal::Center)
        .into()
}

fn install_stage_segment(state: StepVisualState) -> Element<'static, Message> {
    container(text(""))
        .width(Length::FillPortion(1))
        .height(STEPPER_LINE_HEIGHT)
        .style(move |theme| step_connector_style(theme, state))
        .into()
}

fn install_stage_half_segment(state: Option<StepVisualState>) -> Element<'static, Message> {
    let mut segment = container(text(""))
        .width(Length::FillPortion(1))
        .height(STEPPER_LINE_HEIGHT);

    segment = if let Some(state) = state {
        segment.style(move |theme| step_connector_style(theme, state))
    } else {
        segment.style(empty_connector_style)
    };

    segment.into()
}

fn install_stage_gap() -> Element<'static, Message> {
    container(text(""))
        .width(Length::FillPortion(1))
        .height(STEPPER_LINE_HEIGHT)
        .into()
}

fn stepper_edge_spacer() -> Element<'static, Message> {
    container(text(""))
        .width(STEPPER_EDGE_INSET)
        .height(STEPPER_LINE_HEIGHT)
        .into()
}

fn install_log_panel<'a>(
    task: &'a InstallTaskView,
    language: AppLanguage,
    dark: bool,
) -> Element<'a, Message> {
    let mut logs = column![
        text(i18n::live_logs_title(language))
            .size(15)
            .color(muted_text_color(dark))
    ]
    .spacing(8);

    if task.logs.is_empty() {
        logs = logs.push(
            text(i18n::empty_install_logs(language))
                .size(14)
                .color(muted_text_color(dark)),
        );
    } else {
        for log in &task.logs {
            logs = logs.push(
                text(format!("› {log}"))
                    .size(14)
                    .color(detail_text_color(dark)),
            );
        }
    }

    container(logs)
        .padding(16)
        .width(Length::Fill)
        .style(section_card)
        .into()
}

fn install_stage_states(
    platform: Platform,
    task: &InstallTaskView,
    report: Option<&DoctorReport>,
) -> [StepVisualState; 4] {
    match &task.state {
        InstallState::Running => sequential_active_states(active_install_stage(platform, task)),
        InstallState::Opening => sequential_active_states(3),
        InstallState::Completed => [StepVisualState::Done; 4],
        InstallState::Failed(error) => {
            sequential_blocked_states(failed_install_stage(platform, error))
        }
        InstallState::Idle => report
            .map(|report| report_stage_states(platform, report))
            .unwrap_or([StepVisualState::Pending; 4]),
    }
}

fn sequential_active_states(active: usize) -> [StepVisualState; 4] {
    let mut states = [StepVisualState::Pending; 4];
    for (index, state) in states.iter_mut().enumerate() {
        *state = if index < active {
            StepVisualState::Done
        } else if index == active {
            StepVisualState::Active
        } else {
            StepVisualState::Pending
        };
    }
    states
}

fn sequential_blocked_states(blocked: usize) -> [StepVisualState; 4] {
    let mut states = [StepVisualState::Pending; 4];
    for (index, state) in states.iter_mut().enumerate() {
        *state = if index < blocked {
            StepVisualState::Done
        } else if index == blocked {
            StepVisualState::Blocked
        } else {
            StepVisualState::Pending
        };
    }
    states
}

fn report_stage_states(platform: Platform, report: &DoctorReport) -> [StepVisualState; 4] {
    let readiness = match platform {
        Platform::Ios => [
            doctor_check_ready(report, "xcode_app"),
            doctor_check_ready(report, "xcodebuild"),
            doctor_check_ready(report, "xcode_license"),
            doctor_check_ready(report, "ios_runtime"),
        ],
        Platform::Android => [
            doctor_check_ready(report, "sdk_root"),
            doctor_check_ready(report, "java_runtime"),
            doctor_check_ready(report, "sdkmanager")
                && doctor_check_ready(report, "avdmanager")
                && doctor_check_ready(report, "emulator")
                && doctor_check_ready(report, "adb"),
            doctor_check_ready(report, "system_images"),
        ],
    };

    if readiness.iter().all(|ready| *ready) {
        return [StepVisualState::Done; 4];
    }

    let blocked = readiness
        .iter()
        .position(|ready| !*ready)
        .unwrap_or(readiness.len().saturating_sub(1));
    sequential_blocked_states(blocked)
}

fn doctor_check_ready(report: &DoctorReport, key: &str) -> bool {
    report
        .checks
        .iter()
        .find(|check| check.key == key)
        .is_some_and(|check| check.ready)
}

fn active_install_stage(platform: Platform, task: &InstallTaskView) -> usize {
    let current = task.current_step.to_lowercase();

    match platform {
        Platform::Ios => {
            if current.contains("license") || current.contains("许可证") || current.contains("授权")
            {
                2
            } else if current.contains("runtime")
                || current.contains("simulator")
                || current.contains("download")
                || current.contains("platform")
                || current.contains("boot")
                || current.contains("运行时")
                || current.contains("模拟器")
                || current.contains("下载")
                || current.contains("启动")
            {
                3
            } else if current.contains("runfirstlaunch")
                || current.contains("xcodebuild")
                || current.contains("simctl")
                || current.contains("首次")
                || current.contains("工具")
            {
                1
            } else if task.progress < 15.0 {
                0
            } else if task.progress < 68.0 {
                1
            } else {
                3
            }
        }
        Platform::Android => {
            if current.contains("java") {
                1
            } else if current.contains("sdkmanager")
                || current.contains("avdmanager")
                || current.contains("command-line")
                || current.contains("platform-tools")
                || current.contains("命令行")
                || current.contains("工具")
            {
                2
            } else if current.contains("system image")
                || current.contains("emulator")
                || current.contains("avd")
                || current.contains("镜像")
                || current.contains("模拟器")
            {
                3
            } else if task.progress < 25.0 {
                0
            } else if task.progress < 40.0 {
                1
            } else if task.progress < 84.0 {
                2
            } else {
                3
            }
        }
    }
}

fn failed_install_stage(platform: Platform, error: &str) -> usize {
    let error = error.to_lowercase();

    match platform {
        Platform::Ios => {
            if error.contains("xcode.app") || error.contains("xcode developer directory") {
                0
            } else if error.contains("license") || error.contains("许可证") {
                2
            } else if error.contains("xcodebuild") || error.contains("simctl") {
                1
            } else {
                3
            }
        }
        Platform::Android => {
            if error.contains("java") {
                1
            } else if error.contains("sdkmanager")
                || error.contains("avdmanager")
                || error.contains("adb")
                || error.contains("tool")
            {
                2
            } else if error.contains("image") || error.contains("emulator") || error.contains("avd")
            {
                3
            } else {
                0
            }
        }
    }
}

fn is_xcode_license_error(error: &str) -> bool {
    error.contains("Xcode license has not been accepted")
        || error.contains("You have not agreed to the Xcode license agreements")
}

fn banner_style(theme: &Theme, ready: bool) -> container::Style {
    let palette = theme.extended_palette();
    let pair = if ready {
        palette.primary.weak
    } else {
        palette.danger.weak
    };

    container::Style::default()
        .background(pair.color)
        .color(pair.text)
        .border(border::rounded(18).color(pair.color).width(1.0))
}

fn section_card(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style::default()
        .background(palette.background.weak.color)
        .color(palette.background.weak.text)
        .border(
            border::rounded(18)
                .color(palette.background.strong.color)
                .width(1.0),
        )
}

fn main_scrollbar_style(
    theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let is_dark = theme.extended_palette().is_dark;
    let base_scroller = if is_dark {
        Color::from_rgb8(0x7F, 0x8A, 0x96)
    } else {
        Color::from_rgb8(0xB7, 0xB7, 0xB7)
    };
    let hover_scroller = if is_dark {
        Color::from_rgb8(0xA2, 0xAC, 0xB8)
    } else {
        Color::from_rgb8(0x9E, 0x9E, 0x9E)
    };
    let drag_scroller = if is_dark {
        Color::from_rgb8(0xC4, 0xCC, 0xD6)
    } else {
        Color::from_rgb8(0x82, 0x82, 0x82)
    };
    let scroller_color = match status {
        iced::widget::scrollable::Status::Active => base_scroller,
        iced::widget::scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered,
            ..
        } => {
            if is_vertical_scrollbar_hovered {
                hover_scroller
            } else {
                base_scroller
            }
        }
        iced::widget::scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged,
            ..
        } => {
            if is_vertical_scrollbar_dragged {
                drag_scroller
            } else {
                base_scroller
            }
        }
    };
    let rail = iced::widget::scrollable::Rail {
        background: None,
        border: border::rounded(999),
        scroller: iced::widget::scrollable::Scroller {
            color: scroller_color,
            border: border::rounded(999),
        },
    };

    iced::widget::scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
    }
}

fn pill_style(_theme: &Theme, color: Color) -> container::Style {
    container::Style::default()
        .background(color)
        .color(Color::WHITE)
        .border(border::rounded(999))
}

fn step_color(state: StepVisualState, dark: bool) -> Color {
    match (state, dark) {
        (StepVisualState::Done | StepVisualState::Active, _) => Color::from_rgb8(0x2B, 0x87, 0xFF),
        (StepVisualState::Blocked, _) => Color::from_rgb8(0xE0, 0x6A, 0x57),
        (StepVisualState::Pending, true) => Color::from_rgb8(0x7C, 0x86, 0x93),
        (StepVisualState::Pending, false) => Color::from_rgb8(0x9A, 0xA3, 0xAE),
    }
}

fn step_label_color(state: StepVisualState, dark: bool) -> Color {
    match state {
        StepVisualState::Done | StepVisualState::Active => primary_text_color(dark),
        StepVisualState::Blocked => step_color(state, dark),
        StepVisualState::Pending => muted_text_color(dark),
    }
}

fn stepper_card_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    let is_dark = palette.is_dark;
    let background = if is_dark {
        Color::from_rgb8(0x11, 0x17, 0x20)
    } else {
        Color::from_rgb8(0xFF, 0xFB, 0xF3)
    };
    let border_color = if is_dark {
        Color::from_rgb8(0x2A, 0x34, 0x42)
    } else {
        Color::from_rgb8(0xDD, 0xD6, 0xCB)
    };

    container::Style::default()
        .background(background)
        .color(palette.background.base.text)
        .border(border::rounded(18).color(border_color).width(1.0))
}

fn step_dot_style(theme: &Theme, state: StepVisualState, spinner_frame: usize) -> container::Style {
    let is_dark = theme.extended_palette().is_dark;
    let mut color = step_color(state, is_dark);

    if state == StepVisualState::Active && spinner_frame % 6 >= 3 {
        color.a = 0.45;
    }

    container::Style::default()
        .background(color)
        .border(border::rounded(999))
}

fn step_connector_style(theme: &Theme, state: StepVisualState) -> container::Style {
    let is_dark = theme.extended_palette().is_dark;
    let color = match state {
        StepVisualState::Done | StepVisualState::Active => Color::from_rgb8(0x2B, 0x87, 0xFF),
        StepVisualState::Blocked | StepVisualState::Pending if is_dark => {
            Color::from_rgb8(0x4B, 0x4F, 0x56)
        }
        StepVisualState::Blocked | StepVisualState::Pending => Color::from_rgb8(0xC8, 0xC8, 0xC8),
    };

    container::Style::default()
        .background(color)
        .border(border::rounded(999))
}

fn empty_connector_style(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::TRANSPARENT)
        .border(border::rounded(999))
}

fn inactive_tab_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(0xE7, 0xF1, 0xEF),
        iced::widget::button::Status::Pressed => Color::from_rgb8(0xD5, 0xE8, 0xE5),
        _ => Color::from_rgb8(0xF2, 0xEE, 0xE5),
    };

    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: Color::from_rgb8(0x0D, 0x74, 0x7A),
        border: border::rounded(999)
            .color(Color::from_rgb8(0x0D, 0x74, 0x7A))
            .width(1.0),
        ..Default::default()
    }
}

fn primary_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(0x12, 0x8E, 0x95),
        iced::widget::button::Status::Pressed => Color::from_rgb8(0x0A, 0x61, 0x66),
        iced::widget::button::Status::Disabled => Color::from_rgb8(0x84, 0x98, 0x9A),
        iced::widget::button::Status::Active => Color::from_rgb8(0x0D, 0x74, 0x7A),
    };

    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: Color::WHITE,
        border: border::rounded(14),
        ..Default::default()
    }
}

fn secondary_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let dark = theme.extended_palette().is_dark;
    let (background, text_color, border_color) = match (status, dark) {
        (iced::widget::button::Status::Hovered, true) => (
            Color::from_rgb8(0x2A, 0x36, 0x45),
            Color::from_rgb8(0xF5, 0xF7, 0xFA),
            Color::from_rgb8(0xD3, 0xDB, 0xE5),
        ),
        (iced::widget::button::Status::Pressed, true) => (
            Color::from_rgb8(0x1F, 0x2A, 0x38),
            Color::from_rgb8(0xF5, 0xF7, 0xFA),
            Color::from_rgb8(0xB8, 0xC3, 0xD0),
        ),
        (iced::widget::button::Status::Disabled, true) => (
            Color::from_rgb8(0x3A, 0x43, 0x4D),
            Color::from_rgb8(0xA9, 0xB5, 0xC2),
            Color::from_rgb8(0x6C, 0x76, 0x82),
        ),
        (_, true) => (
            Color::from_rgb8(0x17, 0x22, 0x30),
            Color::from_rgb8(0xEC, 0xF1, 0xF5),
            Color::from_rgb8(0x98, 0xA6, 0xB6),
        ),
        (iced::widget::button::Status::Hovered, false) => (
            Color::from_rgb8(0xFF, 0xFB, 0xF3),
            Color::from_rgb8(0x0D, 0x74, 0x7A),
            Color::from_rgb8(0x0D, 0x74, 0x7A),
        ),
        (iced::widget::button::Status::Pressed, false) => (
            Color::from_rgb8(0xEE, 0xE7, 0xDA),
            Color::from_rgb8(0x0A, 0x61, 0x66),
            Color::from_rgb8(0x0A, 0x61, 0x66),
        ),
        (iced::widget::button::Status::Disabled, false) => (
            Color::from_rgb8(0xE1, 0xDD, 0xD4),
            Color::from_rgb8(0x7A, 0x83, 0x8F),
            Color::from_rgb8(0xB4, 0xAE, 0xA3),
        ),
        (_, false) => (
            Color::from_rgb8(0xF6, 0xF1, 0xE8),
            Color::from_rgb8(0x0D, 0x74, 0x7A),
            Color::from_rgb8(0x0D, 0x74, 0x7A),
        ),
    };

    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color,
        border: border::rounded(14).color(border_color).width(1.0),
        ..Default::default()
    }
}

fn load_doctor_task() -> Task<Message> {
    Task::perform(load_doctor_snapshot(), Message::DoctorLoaded)
}

async fn open_simulator(platform: Platform) -> Result<(), String> {
    match platform {
        Platform::Ios => open_ios_simulator().await,
        Platform::Android => open_android_emulator().await,
    }
}

async fn open_ios_simulator() -> Result<(), String> {
    let output = if let Some(developer_dir) = discover_xcode_developer_dir() {
        let simulator_app = developer_dir.join("Applications/Simulator.app");
        tokio::process::Command::new("open")
            .arg(simulator_app)
            .output()
            .await
    } else {
        tokio::process::Command::new("open")
            .args(["-a", "Simulator"])
            .output()
            .await
    }
    .map_err(|error| error.to_string())?;

    command_result(&output, "open Simulator.app")
}

async fn open_android_emulator() -> Result<(), String> {
    let paths = AppPaths::detect().map_err(|error| error.to_string())?;
    let emulator_path = paths.android_sdk_root.join("emulator/emulator");
    let emulator_program = if emulator_path.exists() {
        emulator_path
    } else {
        PathBuf::from("emulator")
    };

    let list_output = tokio::process::Command::new(&emulator_program)
        .arg("-list-avds")
        .output()
        .await
        .map_err(|error| error.to_string())?;
    command_result(&list_output, "emulator -list-avds")?;

    let avd_name = String::from_utf8_lossy(&list_output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "No Android virtual device was found to open".to_string())?;

    tokio::process::Command::new(&emulator_program)
        .args(["-avd", &avd_name])
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn command_result(output: &std::process::Output, command: &str) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary = first_non_empty_line(&stderr)
        .or_else(|| first_non_empty_line(&stdout))
        .unwrap_or("command returned no output");

    Err(format!("{command} failed: {summary}"))
}

async fn accept_xcode_license(language: AppLanguage) -> Result<XcodeLicenseOutcome, String> {
    let developer_dir = discover_xcode_developer_dir()
        .ok_or_else(|| "No Xcode.app installation found in /Applications".to_string())?;
    let xcodebuild_path = developer_dir.join("usr/bin/xcodebuild");

    if !xcodebuild_path.exists() {
        return Err(format!(
            "xcodebuild was not found at {}",
            xcodebuild_path.display()
        ));
    }

    let shell_command = format!(
        "DEVELOPER_DIR={} {} -license accept",
        shell_quote_path(&developer_dir),
        shell_quote_path(&xcodebuild_path)
    );
    let dialog_text = i18n::xcode_license_dialog_text(&shell_command, language);
    let continue_label = i18n::continue_label(language);
    let cancel_label = i18n::cancel_label(language);

    let display_dialog = format!(
        "display dialog \"{}\" buttons {{\"{}\", \"{}\"}} default button \"{}\" cancel button \"{}\" with icon caution",
        applescript_escape(&dialog_text),
        applescript_escape(cancel_label),
        applescript_escape(continue_label),
        applescript_escape(continue_label),
        applescript_escape(cancel_label),
    );
    let run_command = format!(
        "do shell script \"{}\" with administrator privileges",
        applescript_escape(&shell_command)
    );

    let output = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(display_dialog)
        .arg("-e")
        .arg(run_command)
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if output.status.success() {
        return Ok(XcodeLicenseOutcome::Accepted);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let summary = first_non_empty_line(&stderr)
        .or_else(|| first_non_empty_line(&stdout))
        .unwrap_or("osascript returned no output")
        .to_string();

    if summary.contains("User canceled") || summary.contains("-128") {
        Ok(XcodeLicenseOutcome::Cancelled)
    } else {
        Err(summary)
    }
}

fn discover_xcode_developer_dir() -> Option<PathBuf> {
    let applications_dir = Path::new("/Applications");
    let entries = fs::read_dir(applications_dir).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension() == Some(OsStr::new("app"))
                && path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("Xcode"))
        })
        .collect::<Vec<_>>();

    candidates.sort();
    candidates
        .iter()
        .find(|path| path.file_name() == Some(OsStr::new("Xcode.app")))
        .cloned()
        .or_else(|| candidates.into_iter().next())
        .map(|path| path.join("Contents/Developer"))
        .filter(|path| path.exists())
}

fn shell_quote_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn first_non_empty_line(output: &str) -> Option<&str> {
    output.lines().map(str::trim).find(|line| !line.is_empty())
}

fn run_install_stream(platform: Platform) -> impl iced::futures::Stream<Item = InstallStreamEvent> {
    stream::channel(32, move |mut output| async move {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<TaskEvent>();

        let install_task = tokio::spawn(async move {
            match platform {
                Platform::Ios => {
                    let provider = IosProvider::new();
                    provider
                        .install_runtime(default_install_request(platform), Some(sender))
                        .await
                }
                Platform::Android => {
                    let paths = AppPaths::detect().map_err(|error| error.to_string())?;
                    let provider = AndroidProvider::new(paths.android_sdk_root);
                    provider
                        .install_runtime(default_install_request(platform), Some(sender))
                        .await
                }
            }
            .map_err(|error| error.to_string())
        });

        while let Some(event) = receiver.recv().await {
            if output.send(InstallStreamEvent::Event(event)).await.is_err() {
                return;
            }
        }

        let result = match install_task.await {
            Ok(result) => result,
            Err(error) => Err(error.to_string()),
        };

        let _ = output.send(InstallStreamEvent::Finished(result)).await;
    })
}

fn default_install_request(platform: Platform) -> InstallRequest {
    match platform {
        Platform::Ios => InstallRequest {
            platform,
            runtime_version: "18.0".to_string(),
            device_name: Some("iPhone 16".to_string()),
        },
        Platform::Android => InstallRequest {
            platform,
            runtime_version: "35".to_string(),
            device_name: Some("pixel_8".to_string()),
        },
    }
}

fn report_for_platform(snapshot: &DoctorSnapshot, platform: Platform) -> Option<&DoctorReport> {
    snapshot
        .reports
        .iter()
        .find(|report| report.platform == platform)
}

async fn load_doctor_snapshot() -> Result<DoctorSnapshot, String> {
    let paths = AppPaths::detect().map_err(|error| error.to_string())?;
    let ios = IosProvider::new();
    let android = AndroidProvider::new(paths.android_sdk_root.clone());
    let service = SimdockService::new(ios, android);
    let reports = service
        .doctor_all()
        .await
        .map_err(|error| error.to_string())?;

    Ok(DoctorSnapshot { reports })
}
