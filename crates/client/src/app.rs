use crownveil_content::{load_events, load_world_with_map, MapLayout, MapPosition};
use crownveil_sim::{
    food_capacity, ActiveModifier, Command, FactionId, Province, ProvinceId, World,
};
use eframe::egui;

const WORLD_SOURCE: &str = include_str!("../../../content/world.ron");
const EVENTS_SOURCE: &str = include_str!("../../../content/events.ron");
const CHRONICLE_ENTRIES: usize = 12;
const SECONDS_PER_MONTH: f64 = 0.5;
const MAX_MONTHS_PER_FRAME: u32 = 24;
const HEX_RADIUS: f32 = 50.0;
const MIN_ZOOM: f32 = 0.35;
const MAX_ZOOM: f32 = 4.5;

const FACTION_COLORS: [egui::Color32; 6] = [
    egui::Color32::from_rgb(84, 130, 200),
    egui::Color32::from_rgb(190, 80, 70),
    egui::Color32::from_rgb(90, 160, 90),
    egui::Color32::from_rgb(180, 150, 60),
    egui::Color32::from_rgb(150, 90, 180),
    egui::Color32::from_rgb(80, 160, 170),
];

const UNREST_LOW: egui::Color32 = egui::Color32::from_rgb(70, 160, 90);
const UNREST_MID: egui::Color32 = egui::Color32::from_rgb(220, 190, 70);
const UNREST_HIGH: egui::Color32 = egui::Color32::from_rgb(200, 70, 60);
const POP_LOW: egui::Color32 = egui::Color32::from_rgb(45, 65, 110);
const POP_HIGH: egui::Color32 = egui::Color32::from_rgb(130, 190, 250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Owner,
    Unrest,
    Population,
}

impl Overlay {
    fn label(self) -> &'static str {
        match self {
            Self::Owner => "Dono",
            Self::Unrest => "Descontentamento",
            Self::Population => "População",
        }
    }

    fn legend(self) -> &'static str {
        match self {
            Self::Owner => "cor = facção",
            Self::Unrest => "verde = baixo · vermelho = alto",
            Self::Population => "azul claro = maior população",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MapView {
    offset: egui::Vec2,
    zoom: f32,
    overlay: Overlay,
    fit_requested: bool,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
            overlay: Overlay::Owner,
            fit_requested: true,
        }
    }
}

impl MapView {
    fn fit(&mut self, rect: egui::Rect, world_centers: &[(ProvinceId, egui::Pos2)]) {
        let Some((_, first)) = world_centers.first() else {
            self.offset = egui::Vec2::ZERO;
            self.zoom = 1.0;
            return;
        };

        let mut min = *first;
        let mut max = *first;
        for (_, center) in world_centers {
            min.x = min.x.min(center.x);
            min.y = min.y.min(center.y);
            max.x = max.x.max(center.x);
            max.y = max.y.max(center.y);
        }

        let span = max - min;
        let margin = 80.0;
        let available_width = (rect.width() - margin).max(1.0);
        let available_height = (rect.height() - margin).max(1.0);
        let zoom_x = available_width / (span.x.max(1.0) + HEX_RADIUS * 2.0);
        let zoom_y = available_height / (span.y.max(1.0) + HEX_RADIUS * 2.0);

        self.zoom = zoom_x.min(zoom_y).clamp(MIN_ZOOM, MAX_ZOOM);
        let center = min + (max - min) * 0.5;
        self.offset = -center.to_vec2() * self.zoom;
    }

    fn handle_input(&mut self, ui: &egui::Ui, response: &egui::Response, rect: egui::Rect) {
        if response.dragged() {
            self.offset += response.drag_delta();
        }

        let Some(pointer) = response.hover_pos() else {
            return;
        };

        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        let pinch = ui.input(|input| input.zoom_delta());
        let mut factor = 1.0;
        if scroll.abs() > f32::EPSILON {
            factor *= (scroll * 0.002).exp();
        }
        if (pinch - 1.0).abs() > f32::EPSILON {
            factor *= pinch;
        }
        if (factor - 1.0).abs() <= f32::EPSILON {
            return;
        }

        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new_zoom - self.zoom).abs() <= f32::EPSILON {
            return;
        }

        let world = (pointer - rect.center() - self.offset) / self.zoom;
        self.offset = pointer - rect.center() - world * new_zoom;
        self.zoom = new_zoom;
    }

    fn zoom_by(&mut self, factor: f32) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let scale = new_zoom / self.zoom;
        self.offset *= scale;
        self.zoom = new_zoom;
    }

    fn to_screen(self, rect: egui::Rect, world: egui::Pos2) -> egui::Pos2 {
        rect.center() + self.offset + world.to_vec2() * self.zoom
    }
}

pub struct CrownveilApp {
    world: World,
    map_layout: MapLayout,
    player: FactionId,
    paused: bool,
    speed: u32,
    accumulator: f64,
    selected_province: Option<ProvinceId>,
    war_target: FactionId,
    status: String,
    map_view: MapView,
}

impl CrownveilApp {
    pub fn new() -> Self {
        let loaded = load_world_with_map(WORLD_SOURCE, 1).and_then(|(world, map_layout)| {
            load_events(EVENTS_SOURCE, world).map(|world| (world, map_layout))
        });
        match loaded {
            Ok((world, map_layout)) => Self::with_world_and_map(world, map_layout),
            Err(error) => {
                let world = World::demo(1);
                let map_layout = MapLayout::grid(world.provinces.len());
                let mut app = Self::with_world_and_map(world, map_layout);
                app.status = format!("Falha ao carregar conteúdo: {error}. Usando mundo demo.");
                app
            }
        }
    }

    #[cfg(test)]
    pub fn with_world(world: World) -> Self {
        let map_layout = MapLayout::grid(world.provinces.len());
        Self::with_world_and_map(world, map_layout)
    }

    pub fn with_world_and_map(world: World, map_layout: MapLayout) -> Self {
        let war_target = if world.factions.len() > 1 { 1 } else { 0 };
        Self {
            world,
            map_layout,
            player: 0,
            paused: true,
            speed: 1,
            accumulator: 0.0,
            selected_province: None,
            war_target,
            status: "Pausado. Use Continuar ou +1 mês.".to_string(),
            map_view: MapView::default(),
        }
    }

    pub fn advance(&mut self, delta_seconds: f64) {
        if self.paused {
            return;
        }

        let max_accumulator = SECONDS_PER_MONTH * MAX_MONTHS_PER_FRAME as f64;
        self.accumulator =
            (self.accumulator + delta_seconds * self.speed as f64).min(max_accumulator);

        let months = (self.accumulator / SECONDS_PER_MONTH).floor() as u32;
        if months == 0 {
            return;
        }

        for _ in 0..months {
            self.world.tick_month();
        }
        self.accumulator -= months as f64 * SECONDS_PER_MONTH;
    }

    fn step_once(&mut self) {
        self.world.tick_month();
        self.accumulator = 0.0;
    }

    fn normalize_target(&mut self) {
        if self.war_target == self.player {
            if let Some(next) = (0..self.world.factions.len()).find(|id| *id != self.player) {
                self.war_target = next;
            }
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Crownveil");
                ui.separator();

                let play_label = if self.paused { "Continuar" } else { "Pausar" };
                if ui.button(play_label).clicked() {
                    self.paused = !self.paused;
                }
                if ui.button("+1 mês").clicked() {
                    self.step_once();
                }

                for speed in [1, 2, 4, 8] {
                    if ui
                        .selectable_label(self.speed == speed, format!("{speed}x"))
                        .clicked()
                    {
                        self.speed = speed;
                    }
                }

                ui.separator();
                ui.label(format!(
                    "Ano {} / mês {}",
                    self.world.year, self.world.month
                ));
                ui.separator();
                ui.label(format!(
                    "Guerras ativas: {}",
                    self.world.diplomacy.wars.len()
                ));
                ui.separator();

                let player_name = self.world.factions[self.player].name.clone();
                egui::ComboBox::from_label("Jogador")
                    .selected_text(player_name)
                    .show_ui(ui, |ui| {
                        for index in 0..self.world.factions.len() {
                            let faction = &self.world.factions[index];
                            ui.selectable_value(
                                &mut self.player,
                                faction.id,
                                faction.name.as_str(),
                            );
                        }
                    });
            });
        });
    }

    fn factions_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("factions_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Facções");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for index in 0..self.world.factions.len() {
                        let (id, name, treasury, legitimacy, tax_rate, army) = {
                            let faction = &self.world.factions[index];
                            (
                                faction.id,
                                faction.name.clone(),
                                faction.treasury,
                                faction.legitimacy,
                                faction.tax_rate,
                                faction.army,
                            )
                        };
                        let is_player = id == self.player;
                        let provinces = self.world.province_count(id);
                        let wars = self.world.diplomacy.war_count_for(id);
                        let modifiers = self.world.factions[index].modifiers.clone();

                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(name);
                                if is_player {
                                    ui.label("(jogador)");
                                }
                            });
                            ui.label(format!("Tesouro: {treasury}"));
                            ui.label(format!("Legitimidade: {legitimacy}"));
                            ui.label(format!("Imposto: {tax_rate}%"));
                            ui.label(format!("Exército: {army}"));
                            ui.label(format!("Províncias: {provinces}"));
                            ui.label(format!("Guerras: {wars}"));
                            modifier_list(ui, &self.world, &modifiers);
                            if !is_player && ui.button("Jogar como esta facção").clicked() {
                                self.player = id;
                            }
                        });
                    }
                });
            });
    }

    fn player_actions(&mut self, ui: &mut egui::Ui) {
        self.normalize_target();

        let player = self.player;
        let player_name = self.world.factions[player].name.clone();
        ui.horizontal(|ui| {
            ui.strong(format!("Ações — {player_name}"));

            if ui.button("Imposto -5").clicked() {
                let next = (self.world.factions[player].tax_rate - 5).clamp(0, 100);
                self.world.queue_command(Command::SetTax {
                    faction: player,
                    rate: next,
                });
                self.status = format!("Imposto definido para {next}% (próximo mês)");
            }
            if ui.button("Imposto +5").clicked() {
                let next = (self.world.factions[player].tax_rate + 5).clamp(0, 100);
                self.world.queue_command(Command::SetTax {
                    faction: player,
                    rate: next,
                });
                self.status = format!("Imposto definido para {next}% (próximo mês)");
            }
            if ui.button("Recrutar 100").clicked() {
                self.world.queue_command(Command::Recruit {
                    faction: player,
                    soldiers: 100,
                });
                self.status = "Recrutamento de 100 soldados enfileirado".to_string();
            }
            if ui.button("Dispensar 100").clicked() {
                self.world.queue_command(Command::Disband {
                    faction: player,
                    soldiers: 100,
                });
                self.status = "Dispensa de 100 soldados enfileirada".to_string();
            }

            let target_name = self.world.factions[self.war_target].name.clone();
            egui::ComboBox::from_label("Alvo")
                .selected_text(target_name)
                .show_ui(ui, |ui| {
                    for index in 0..self.world.factions.len() {
                        let faction = &self.world.factions[index];
                        if faction.id != player {
                            ui.selectable_value(
                                &mut self.war_target,
                                faction.id,
                                faction.name.as_str(),
                            );
                        }
                    }
                });

            let target = self.war_target;
            if self.world.diplomacy.is_at_war(player, target) {
                if ui.button("Fazer paz").clicked() {
                    self.world.queue_command(Command::MakePeace {
                        faction: player,
                        other: target,
                    });
                    self.status = "Paz enfileirada".to_string();
                }
            } else {
                let truce = self.world.diplomacy.truce_months(player, target);
                if truce > 0 {
                    ui.label(format!("Trégua: {truce} meses"));
                } else if ui.button("Declarar guerra").clicked() {
                    self.world.queue_command(Command::DeclareWar {
                        attacker: player,
                        defender: target,
                    });
                    self.status = "Guerra declarada (aplicada no próximo mês)".to_string();
                }
            }
        });
    }

    fn map_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Camada:");
            egui::ComboBox::from_id_salt("overlay")
                .selected_text(self.map_view.overlay.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.map_view.overlay, Overlay::Owner, "Dono");
                    ui.selectable_value(
                        &mut self.map_view.overlay,
                        Overlay::Unrest,
                        "Descontentamento",
                    );
                    ui.selectable_value(
                        &mut self.map_view.overlay,
                        Overlay::Population,
                        "População",
                    );
                });
            if ui.button("Centrar").clicked() {
                self.map_view.fit_requested = true;
            }
            if ui.button("−").clicked() {
                self.map_view.zoom_by(1.2);
            }
            if ui.button("+").clicked() {
                self.map_view.zoom_by(1.0 / 1.2);
            }
            ui.separator();
            ui.label(self.map_view.overlay.legend());
        });
    }

    fn map_panel(&mut self, ui: &mut egui::Ui) {
        self.map_toolbar(ui);
        ui.add_space(4.0);

        let available = ui.available_size();
        let size = egui::vec2(available.x.max(1.0), available.y.max(1.0));
        let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 28, 34));

        let world_centers: Vec<(ProvinceId, egui::Pos2)> = self
            .world
            .provinces
            .iter()
            .map(|province| {
                let position = self
                    .map_layout
                    .position(province.id)
                    .unwrap_or(MapPosition { col: 0, row: 0 });
                (province.id, hex_center(position))
            })
            .collect();

        if self.map_view.fit_requested {
            self.map_view.fit(rect, &world_centers);
            self.map_view.fit_requested = false;
        }

        self.map_view.handle_input(ui, &response, rect);

        let zoom = self.map_view.zoom;
        let screen_centers: Vec<(ProvinceId, egui::Pos2)> = world_centers
            .iter()
            .map(|(id, center)| (*id, self.map_view.to_screen(rect, *center)))
            .collect();

        let max_population = self
            .world
            .provinces
            .iter()
            .map(|province| province.pops.total())
            .max()
            .unwrap_or(1);
        let overlay = self.map_view.overlay;
        let selected = self.selected_province;

        for (id, center) in &screen_centers {
            let province = &self.world.provinces[*id];
            let fill = overlay_color(overlay, province, max_population);
            let stroke = if selected == Some(*id) {
                egui::Stroke::new(3.0_f32, egui::Color32::WHITE)
            } else {
                egui::Stroke::new(1.0_f32, egui::Color32::from_gray(35))
            };
            painter.add(egui::Shape::convex_polygon(
                hex_points(*center, HEX_RADIUS * zoom),
                fill,
                stroke,
            ));
        }

        for (id, center) in &screen_centers {
            let province = &self.world.provinces[*id];
            let fill = overlay_color(overlay, province, max_population);
            let text_color = contrast_color(fill);
            let font_size = (11.0 * zoom).clamp(9.0, 20.0);
            let name = shorten(&province.name, 16);
            let text = match overlay {
                Overlay::Owner => name,
                Overlay::Unrest => format!("{name}\n{}", province.unrest),
                Overlay::Population => format!("{name}\n{}", province.pops.total()),
            };
            painter.text(
                *center,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(font_size),
                text_color,
            );
        }

        let hovered = response
            .hover_pos()
            .and_then(|pointer| nearest_province(pointer, &screen_centers, HEX_RADIUS * zoom));

        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                if let Some(id) = nearest_province(pointer, &screen_centers, HEX_RADIUS * zoom) {
                    self.selected_province = Some(id);
                    self.status = format!("Selecionada: {}", self.world.provinces[id].name);
                }
            }
        }

        let tooltip = hovered.map(|id| {
            let province = &self.world.provinces[id];
            let owner = self
                .world
                .faction(province.owner)
                .map(|faction| faction.name.clone())
                .unwrap_or_else(|| "?".to_string());
            (
                province.name.clone(),
                owner,
                province.pops.total(),
                food_capacity(province),
                province.unrest,
                province.development,
            )
        });

        response.on_hover_ui_at_pointer(|ui| {
            if let Some((name, owner, population, capacity, unrest, development)) = &tooltip {
                ui.strong(name);
                ui.label(format!("Dono: {owner}"));
                ui.label(format!("População: {population} / {capacity}"));
                ui.label(format!("Desenvolvimento: {development}"));
                ui.label(format!("Descontentamento: {unrest}"));
            }
        });
    }

    fn province_details(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_province else {
            ui.label("Clique em uma província no mapa para ver os detalhes.");
            return;
        };

        let Some(province) = self.world.provinces.get(id) else {
            self.selected_province = None;
            return;
        };

        let name = province.name.clone();
        let owner = province.owner;
        let development = province.development;
        let unrest = province.unrest;
        let population = province.pops.total();
        let capacity = food_capacity(province);
        let owner_name = self
            .world
            .faction(owner)
            .map(|faction| faction.name.clone())
            .unwrap_or_else(|| "?".to_string());
        let modifiers = province.modifiers.clone();
        let explanation = self.world.explain_unrest(id);
        let at_war = owner != self.player && self.world.diplomacy.is_at_war(self.player, owner);

        ui.heading(name.as_str());
        ui.label(format!("Dono: {owner_name}"));
        ui.label(format!("Desenvolvimento: {development}"));
        ui.label(format!("População: {population} / capacidade {capacity}"));
        ui.label(format!("Descontentamento: {unrest}"));

        if let Some(explanation) = explanation {
            let breakdown = explanation.breakdown;
            ui.label(format!("Alvo de descontentamento: {}", explanation.target));
            ui.label(format!(
                "Causas: legitimidade +{}, tesouro +{}, imposto +{}, comida +{}, guerra +{}, modificadores {:+}",
                breakdown.legitimacy,
                breakdown.treasury,
                breakdown.tax,
                breakdown.food,
                breakdown.war,
                breakdown.modifiers
            ));
        }

        if !modifiers.is_empty() {
            ui.separator();
            ui.strong("Modificadores");
            modifier_list(ui, &self.world, &modifiers);
        }

        if at_war && ui.button("Atacar província").clicked() {
            self.world.queue_command(Command::Attack {
                faction: self.player,
                province: id,
            });
            self.status = format!("Ataque a {name} enfileirado");
        }
    }

    fn details_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("province_details")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.heading("Província selecionada");
                ui.separator();
                self.province_details(ui);
                ui.separator();
                self.chronicle(ui);
            });
    }

    fn chronicle(&mut self, ui: &mut egui::Ui) {
        ui.heading("Crônica");
        if self.world.event_log.is_empty() {
            ui.label("Nenhum evento ainda.");
            return;
        }

        let mut selected = None;
        egui::ScrollArea::vertical()
            .id_salt("chronicle")
            .show(ui, |ui| {
                for record in self.world.event_log.iter().rev().take(CHRONICLE_ENTRIES) {
                    let Some(event) = self.world.event_def(record.event) else {
                        continue;
                    };
                    let target = self.world.target_name(record.target);
                    let label = format!(
                        "{}/{} — {}: {}",
                        record.month, record.year, event.title, target
                    );
                    let response = ui.selectable_label(false, label);
                    let response = if event.description.is_empty() {
                        response
                    } else {
                        response.on_hover_text(event.description.as_str())
                    };
                    if response.clicked() {
                        if let crownveil_sim::EventTarget::Province(id) = record.target {
                            selected = Some(id);
                        }
                    }
                }
            });
        if selected.is_some() {
            self.selected_province = selected;
        }
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.player_actions(ui);
            ui.separator();
            self.map_panel(ui);
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.status.as_str());
                if !self.world.pending_commands.is_empty() {
                    ui.separator();
                    ui.label(format!(
                        "Comandos pendentes: {}",
                        self.world.pending_commands.len()
                    ));
                }
            });
        });
    }
}

impl eframe::App for CrownveilApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let delta_seconds = ctx.input(|input| input.stable_dt) as f64;
        self.advance(delta_seconds);

        self.top_bar(ctx);
        self.status_bar(ctx);
        self.factions_panel(ctx);
        self.details_panel(ctx);
        self.central_panel(ctx);

        if !self.paused {
            ctx.request_repaint();
        }
    }
}

fn modifier_list(ui: &mut egui::Ui, world: &World, modifiers: &[ActiveModifier]) {
    for active in modifiers {
        let name = world
            .modifier_def(active.modifier)
            .map(|def| def.name.as_str())
            .unwrap_or("?");
        let duration = match active.months_left {
            Some(months) => format!("{months} meses"),
            None => "permanente".to_string(),
        };
        ui.label(format!("• {name} ({duration})"))
            .on_hover_text(describe_effects(active));
    }
}

fn describe_effects(active: &ActiveModifier) -> String {
    let effects = active.effects;
    let parts: Vec<String> = [
        (effects.unrest, "descontentamento", ""),
        (effects.income_pct, "receita", "%"),
        (effects.food_pct, "comida", "%"),
        (effects.legitimacy, "legitimidade/mês", ""),
    ]
    .iter()
    .filter(|(value, _, _)| *value != 0)
    .map(|(value, label, suffix)| format!("{label} {value:+}{suffix}"))
    .collect();
    parts.join(", ")
}

fn hex_center(position: MapPosition) -> egui::Pos2 {
    let x = HEX_RADIUS * 3.0_f32.sqrt() * (position.col as f32 + 0.5 * (position.row & 1) as f32);
    let y = HEX_RADIUS * 1.5 * position.row as f32;
    egui::pos2(x, y)
}

fn hex_points(center: egui::Pos2, radius: f32) -> Vec<egui::Pos2> {
    (0..6)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / 6.0 - std::f32::consts::FRAC_PI_6;
            center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
        })
        .collect()
}

fn nearest_province(
    pointer: egui::Pos2,
    centers: &[(ProvinceId, egui::Pos2)],
    radius: f32,
) -> Option<ProvinceId> {
    centers
        .iter()
        .map(|(id, center)| (*id, center.distance(pointer)))
        .filter(|(_, distance)| *distance <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

fn faction_color(id: FactionId) -> egui::Color32 {
    FACTION_COLORS[id % FACTION_COLORS.len()]
}

fn overlay_color(overlay: Overlay, province: &Province, max_population: i64) -> egui::Color32 {
    match overlay {
        Overlay::Owner => faction_color(province.owner),
        Overlay::Unrest => {
            let t = (province.unrest as f32 / 100.0).clamp(0.0, 1.0);
            if t < 0.5 {
                lerp_color(UNREST_LOW, UNREST_MID, t * 2.0)
            } else {
                lerp_color(UNREST_MID, UNREST_HIGH, (t - 0.5) * 2.0)
            }
        }
        Overlay::Population => {
            let t = (province.pops.total() as f32 / max_population.max(1) as f32).clamp(0.0, 1.0);
            lerp_color(POP_LOW, POP_HIGH, t)
        }
    }
}

fn lerp_color(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    egui::Color32::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}

fn contrast_color(color: egui::Color32) -> egui::Color32 {
    let luminance = 0.299 * color.r() as f32 + 0.587 * color.g() as f32 + 0.114 * color.b() as f32;
    if luminance > 140.0 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    }
}

fn shorten(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        name.to_string()
    } else {
        let mut shortened: String = name.chars().take(max_chars.saturating_sub(1)).collect();
        shortened.push('…');
        shortened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_app() -> CrownveilApp {
        CrownveilApp::with_world(World::demo(1))
    }

    #[test]
    fn bundled_content_includes_events() {
        let app = CrownveilApp::new();
        assert!(!app.world.events.is_empty());
        assert!(!app.world.modifiers.is_empty());
    }

    #[test]
    fn effects_description_skips_zero_values() {
        let active = ActiveModifier {
            modifier: 0,
            months_left: Some(3),
            effects: crownveil_sim::ModifierEffects {
                unrest: -4,
                food_pct: 15,
                ..Default::default()
            },
        };
        assert_eq!(
            describe_effects(&active),
            "descontentamento -4, comida +15%"
        );
    }

    #[test]
    fn paused_does_not_advance() {
        let mut app = demo_app();
        app.paused = true;
        app.advance(SECONDS_PER_MONTH * 10.0);
        assert_eq!(app.world.year, 1);
        assert_eq!(app.world.month, 1);
    }

    #[test]
    fn speed_multiplies_months() {
        let mut app = demo_app();
        app.paused = false;
        app.speed = 1;
        app.advance(SECONDS_PER_MONTH * 3.0);
        assert_eq!(app.world.month, 4);

        let mut fast = demo_app();
        fast.paused = false;
        fast.speed = 4;
        fast.advance(SECONDS_PER_MONTH * 3.0);
        assert_eq!(fast.world.year, 2);
        assert_eq!(fast.world.month, 1);
    }

    #[test]
    fn partial_time_accumulates() {
        let mut app = demo_app();
        app.paused = false;
        app.advance(SECONDS_PER_MONTH * 0.5);
        assert_eq!(app.world.month, 1);
        app.advance(SECONDS_PER_MONTH * 0.6);
        assert_eq!(app.world.month, 2);
    }

    #[test]
    fn queued_command_applies_on_next_month() {
        let mut app = demo_app();
        app.paused = false;
        let player = app.player;
        let before = app.world.factions[player].tax_rate;
        app.world.queue_command(Command::SetTax {
            faction: player,
            rate: 55,
        });
        assert_eq!(app.world.factions[player].tax_rate, before);
        app.advance(SECONDS_PER_MONTH);
        assert_eq!(app.world.factions[player].tax_rate, 55);
    }

    #[test]
    fn hex_centers_are_spaced() {
        let a = hex_center(MapPosition { col: 0, row: 0 });
        let b = hex_center(MapPosition { col: 1, row: 0 });
        let expected = HEX_RADIUS * 3.0_f32.sqrt();
        assert!((a.distance(b) - expected).abs() < 0.01);
    }

    #[test]
    fn nearest_province_picks_closest_within_radius() {
        let centers = vec![(0, egui::pos2(0.0, 0.0)), (1, egui::pos2(100.0, 0.0))];
        assert_eq!(
            nearest_province(egui::pos2(5.0, 0.0), &centers, 40.0),
            Some(0)
        );
        assert_eq!(
            nearest_province(egui::pos2(95.0, 0.0), &centers, 40.0),
            Some(1)
        );
        assert_eq!(
            nearest_province(egui::pos2(50.0, 0.0), &centers, 40.0),
            None
        );
    }
}
