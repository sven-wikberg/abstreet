use abstutil::{deserialize_btreemap, read_binary, serialize_btreemap, write_json, Timer};
use dimensioned::si;
use ezgui::{Canvas, Color, GfxCtx, Text};
use geom::{Circle, HashablePt2D, LonLat, PolyLine, Polygon, Pt2D};
use map_model::{raw_data, IntersectionType, LaneType, RoadSpec, LANE_THICKNESS};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::mem;

const INTERSECTION_RADIUS: f64 = 10.0;
const BUILDING_LENGTH: f64 = 30.0;
const CENTER_LINE_THICKNESS: f64 = 0.5;

const HIGHLIGHT_COLOR: Color = Color::CYAN;

pub type BuildingID = usize;
pub type IntersectionID = usize;
pub type RoadID = (IntersectionID, IntersectionID);
pub type Direction = bool;

const FORWARDS: Direction = true;
const BACKWARDS: Direction = false;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub name: Option<String>,
    #[serde(
        serialize_with = "serialize_btreemap",
        deserialize_with = "deserialize_btreemap"
    )]
    intersections: BTreeMap<IntersectionID, Intersection>,
    #[serde(
        serialize_with = "serialize_btreemap",
        deserialize_with = "deserialize_btreemap"
    )]
    roads: BTreeMap<RoadID, Road>,
    #[serde(
        serialize_with = "serialize_btreemap",
        deserialize_with = "deserialize_btreemap"
    )]
    buildings: BTreeMap<BuildingID, Building>,
}

#[derive(Serialize, Deserialize)]
pub struct Intersection {
    center: Pt2D,
    intersection_type: IntersectionType,
    label: Option<String>,
}

impl Intersection {
    fn circle(&self) -> Circle {
        Circle::new(self.center, INTERSECTION_RADIUS)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Road {
    i1: IntersectionID,
    i2: IntersectionID,
    lanes: RoadSpec,
    fwd_label: Option<String>,
    back_label: Option<String>,
}

impl Road {
    fn polygon(&self, direction: Direction, model: &Model) -> Polygon {
        let pl = PolyLine::new(vec![
            model.intersections[&self.i1].center,
            model.intersections[&self.i2].center,
        ]);
        if direction {
            let width = LANE_THICKNESS * (self.lanes.fwd.len() as f64);
            pl.shift_right(width / 2.0).make_polygons(width)
        } else {
            let width = LANE_THICKNESS * (self.lanes.back.len() as f64);
            pl.shift_left(width / 2.0).make_polygons(width)
        }
    }

    fn draw(
        &self,
        model: &Model,
        g: &mut GfxCtx,
        canvas: &Canvas,
        highlight_fwd: bool,
        highlight_back: bool,
    ) {
        let base = PolyLine::new(vec![
            model.intersections[&self.i1].center,
            model.intersections[&self.i2].center,
        ]);

        for (idx, lt) in self.lanes.fwd.iter().enumerate() {
            let polygon = base
                .shift_right(((idx as f64) + 0.5) * LANE_THICKNESS)
                .make_polygons(LANE_THICKNESS);
            g.draw_polygon(
                if highlight_fwd {
                    HIGHLIGHT_COLOR
                } else {
                    Road::lt_to_color(*lt)
                },
                &polygon,
            );
        }
        for (idx, lt) in self.lanes.back.iter().enumerate() {
            let polygon = base
                .shift_left(((idx as f64) + 0.5) * LANE_THICKNESS)
                .make_polygons(LANE_THICKNESS);
            g.draw_polygon(
                if highlight_back {
                    HIGHLIGHT_COLOR
                } else {
                    Road::lt_to_color(*lt)
                },
                &polygon,
            );
        }

        g.draw_polygon(Color::YELLOW, &base.make_polygons(CENTER_LINE_THICKNESS));

        if let Some(ref label) = self.fwd_label {
            canvas.draw_text_at(
                g,
                Text::from_line(label.to_string()),
                self.polygon(FORWARDS, model).center(),
            );
        }
        if let Some(ref label) = self.back_label {
            canvas.draw_text_at(
                g,
                Text::from_line(label.to_string()),
                self.polygon(BACKWARDS, model).center(),
            );
        }
    }

    // Copied from render/lane.rs. :(
    fn lt_to_color(lt: LaneType) -> Color {
        match lt {
            LaneType::Driving => Color::BLACK,
            LaneType::Bus => Color::rgb(190, 74, 76),
            LaneType::Parking => Color::grey(0.2),
            LaneType::Sidewalk => Color::grey(0.8),
            LaneType::Biking => Color::rgb(15, 125, 75),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Building {
    label: Option<String>,
    center: Pt2D,
}

impl Building {
    fn polygon(&self) -> Polygon {
        Polygon::rectangle(self.center, BUILDING_LENGTH, BUILDING_LENGTH)
    }
}

impl Model {
    pub fn new() -> Model {
        Model {
            name: None,
            intersections: BTreeMap::new(),
            roads: BTreeMap::new(),
            buildings: BTreeMap::new(),
        }
    }

    pub fn draw(&self, g: &mut GfxCtx, canvas: &Canvas) {
        g.clear(Color::WHITE);

        let cursor = canvas.get_cursor_in_map_space();
        let current_i = cursor.and_then(|c| self.mouseover_intersection(c));
        let current_b = cursor.and_then(|c| self.mouseover_building(c));
        let current_r = cursor.and_then(|c| self.mouseover_road(c));

        for (id, r) in &self.roads {
            r.draw(
                self,
                g,
                canvas,
                Some((*id, FORWARDS)) == current_r,
                Some((*id, BACKWARDS)) == current_r,
            );
        }

        for (id, i) in &self.intersections {
            let color = if Some(*id) == current_i {
                HIGHLIGHT_COLOR
            } else {
                match i.intersection_type {
                    IntersectionType::TrafficSignal => Color::GREEN,
                    IntersectionType::StopSign => Color::RED,
                    IntersectionType::Border => Color::BLUE,
                }
            };
            g.draw_circle(color, &i.circle());

            if let Some(ref label) = i.label {
                canvas.draw_text_at(g, Text::from_line(label.to_string()), i.center);
            }
        }

        for (id, b) in &self.buildings {
            let color = if Some(*id) == current_b {
                HIGHLIGHT_COLOR
            } else {
                Color::BLUE
            };
            g.draw_polygon(color, &b.polygon());

            if let Some(ref label) = b.label {
                canvas.draw_text_at(g, Text::from_line(label.to_string()), b.center);
            }
        }
    }

    pub fn save(&self) {
        let path = format!(
            "../data/synthetic_maps/{}.json",
            self.name.as_ref().expect("Model hasn't been named yet")
        );
        write_json(&path, self).expect(&format!("Saving {} failed", path));
        println!("Saved {}", path);
    }

    pub fn export(&self) {
        let mut map = raw_data::Map::blank();
        map.coordinates_in_world_space = true;

        fn pt(p: Pt2D) -> LonLat {
            LonLat::new(p.x(), p.y())
        }

        for (idx, r) in self.roads.values().enumerate() {
            let mut osm_tags = BTreeMap::new();
            osm_tags.insert("synthetic_lanes".to_string(), r.lanes.to_string());
            if let Some(ref label) = r.fwd_label {
                osm_tags.insert("fwd_label".to_string(), label.to_string());
            }
            if let Some(ref label) = r.back_label {
                osm_tags.insert("back_label".to_string(), label.to_string());
            }
            map.roads.push(raw_data::Road {
                points: vec![
                    pt(self.intersections[&r.i1].center),
                    pt(self.intersections[&r.i2].center),
                ],
                osm_tags,
                osm_way_id: idx as i64,
                parking_lane_fwd: r.lanes.fwd.contains(&LaneType::Parking),
                parking_lane_back: r.lanes.back.contains(&LaneType::Parking),
            });
        }

        for i in self.intersections.values() {
            map.intersections.push(raw_data::Intersection {
                point: pt(i.center),
                elevation: 0.0 * si::M,
                intersection_type: i.intersection_type,
                label: i.label.clone(),
            });
        }

        for (idx, b) in self.buildings.values().enumerate() {
            let mut osm_tags = BTreeMap::new();
            if let Some(ref label) = b.label {
                osm_tags.insert("label".to_string(), label.to_string());
            }
            map.buildings.push(raw_data::Building {
                // TODO Duplicate points :(
                points: b.polygon().points().into_iter().map(pt).collect(),
                osm_tags,
                osm_way_id: idx as i64,
            });
        }

        let path = format!(
            "../data/raw_maps/{}.abst",
            self.name.as_ref().expect("Model hasn't been named yet")
        );
        abstutil::write_binary(&path, &map).expect(&format!("Saving {} failed", path));
        println!("Exported {}", path);
    }

    // TODO Directly use raw_data and get rid of Model? Might be more maintainable long-term.
    pub fn import(path: &str) -> Model {
        let data: raw_data::Map = read_binary(path, &mut Timer::new("load raw map")).unwrap();
        let gps_bounds = data.get_gps_bounds();

        let mut m = Model::new();
        let mut pt_to_intersection: HashMap<HashablePt2D, IntersectionID> = HashMap::new();

        for (idx, i) in data.intersections.iter().enumerate() {
            let center = Pt2D::from_gps(i.point, &gps_bounds).unwrap();
            m.intersections.insert(
                idx,
                Intersection {
                    center,
                    intersection_type: i.intersection_type,
                    label: i.label.clone(),
                },
            );
            pt_to_intersection.insert(center.into(), idx);
        }

        for r in &data.roads {
            let i1 = pt_to_intersection[&Pt2D::from_gps(r.points[0], &gps_bounds).unwrap().into()];
            let i2 = pt_to_intersection[&Pt2D::from_gps(*r.points.last().unwrap(), &gps_bounds)
                .unwrap()
                .into()];
            m.roads.insert(
                (i1, i2),
                Road {
                    i1,
                    i2,
                    // TODO Do better
                    lanes: RoadSpec {
                        fwd: vec![LaneType::Driving, LaneType::Parking, LaneType::Sidewalk],
                        back: vec![LaneType::Driving, LaneType::Parking, LaneType::Sidewalk],
                    },
                    // TODO nope
                    fwd_label: None,
                    back_label: None,
                },
            );
        }

        for (idx, b) in data.buildings.iter().enumerate() {
            m.buildings.insert(
                idx,
                Building {
                    label: None,
                    center: Pt2D::center(
                        &b.points
                            .iter()
                            .map(|pt| Pt2D::from_gps(*pt, &gps_bounds).unwrap())
                            .collect(),
                    ),
                },
            );
        }

        m
    }
}

impl Model {
    pub fn create_i(&mut self, center: Pt2D) {
        let id = self.intersections.len();
        self.intersections.insert(
            id,
            Intersection {
                center,
                intersection_type: IntersectionType::StopSign,
                label: None,
            },
        );
    }

    pub fn move_i(&mut self, id: IntersectionID, center: Pt2D) {
        self.intersections.get_mut(&id).unwrap().center = center;
    }

    pub fn set_i_label(&mut self, id: IntersectionID, label: String) {
        self.intersections.get_mut(&id).unwrap().label = Some(label);
    }

    pub fn get_i_label(&self, id: IntersectionID) -> Option<String> {
        self.intersections[&id].label.clone()
    }

    pub fn toggle_i_type(&mut self, id: IntersectionID) {
        let i = self.intersections.get_mut(&id).unwrap();
        i.intersection_type = match i.intersection_type {
            IntersectionType::StopSign => IntersectionType::TrafficSignal,
            IntersectionType::TrafficSignal => {
                let num_roads = self
                    .roads
                    .values()
                    .filter(|r| r.i1 == id || r.i2 == id)
                    .count();
                if num_roads == 1 {
                    IntersectionType::Border
                } else {
                    IntersectionType::StopSign
                }
            }
            IntersectionType::Border => IntersectionType::StopSign,
        };
    }

    pub fn remove_i(&mut self, id: IntersectionID) {
        for (i1, i2) in self.roads.keys() {
            if *i1 == id || *i2 == id {
                println!("Can't delete intersection used by roads");
                return;
            }
        }
        self.intersections.remove(&id);
    }

    pub fn get_i_center(&self, id: IntersectionID) -> Pt2D {
        self.intersections[&id].center
    }

    pub fn mouseover_intersection(&self, pt: Pt2D) -> Option<IntersectionID> {
        for (id, i) in &self.intersections {
            if i.circle().contains_pt(pt) {
                return Some(*id);
            }
        }
        None
    }
}

impl Model {
    pub fn create_road(&mut self, i1: IntersectionID, i2: IntersectionID) {
        let id = if i1 < i2 { (i1, i2) } else { (i2, i1) };
        if self.roads.contains_key(&id) {
            println!("Road already exists");
            return;
        }
        self.roads.insert(
            id,
            Road {
                i1,
                i2,
                lanes: RoadSpec {
                    fwd: vec![LaneType::Driving, LaneType::Parking, LaneType::Sidewalk],
                    back: vec![LaneType::Driving, LaneType::Parking, LaneType::Sidewalk],
                },
                fwd_label: None,
                back_label: None,
            },
        );
    }

    pub fn edit_lanes(&mut self, id: RoadID, spec: String) {
        if let Some(s) = RoadSpec::parse(spec.clone()) {
            self.roads.get_mut(&id).unwrap().lanes = s;
        } else {
            println!("Bad RoadSpec: {}", spec);
        }
    }

    pub fn swap_lanes(&mut self, id: RoadID) {
        let lanes = &mut self.roads.get_mut(&id).unwrap().lanes;
        mem::swap(&mut lanes.fwd, &mut lanes.back);
    }

    pub fn set_r_label(&mut self, pair: (RoadID, Direction), label: String) {
        let r = self.roads.get_mut(&pair.0).unwrap();
        if pair.1 {
            r.fwd_label = Some(label);
        } else {
            r.back_label = Some(label);
        }
    }

    pub fn get_r_label(&self, pair: (RoadID, Direction)) -> Option<String> {
        let r = &self.roads[&pair.0];
        if pair.1 {
            r.fwd_label.clone()
        } else {
            r.back_label.clone()
        }
    }

    pub fn remove_road(&mut self, id: RoadID) {
        self.roads.remove(&id);
    }

    pub fn mouseover_road(&self, pt: Pt2D) -> Option<(RoadID, Direction)> {
        for (id, r) in &self.roads {
            if r.polygon(FORWARDS, self).contains_pt(pt) {
                return Some((*id, FORWARDS));
            }
            if r.polygon(BACKWARDS, self).contains_pt(pt) {
                return Some((*id, BACKWARDS));
            }
        }
        None
    }

    pub fn get_lanes(&self, id: RoadID) -> String {
        self.roads[&id].lanes.to_string()
    }
}

impl Model {
    pub fn create_b(&mut self, center: Pt2D) {
        let id = self.buildings.len();
        self.buildings.insert(
            id,
            Building {
                center,
                label: None,
            },
        );
    }

    pub fn move_b(&mut self, id: BuildingID, center: Pt2D) {
        self.buildings.get_mut(&id).unwrap().center = center;
    }

    pub fn set_b_label(&mut self, id: BuildingID, label: String) {
        self.buildings.get_mut(&id).unwrap().label = Some(label);
    }

    pub fn get_b_label(&self, id: BuildingID) -> Option<String> {
        self.buildings[&id].label.clone()
    }

    pub fn remove_b(&mut self, id: BuildingID) {
        self.buildings.remove(&id);
    }

    pub fn mouseover_building(&self, pt: Pt2D) -> Option<BuildingID> {
        for (id, b) in &self.buildings {
            if b.polygon().contains_pt(pt) {
                return Some(*id);
            }
        }
        None
    }
}
