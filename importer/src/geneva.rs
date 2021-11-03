use std::fs;

use geom::LonLat;
use geom::Polygon;
use geojson::{GeoJson, Geometry, Value};
use geom::Pt2D;
use geom::Ring;
use map_model::Map;
use rand_xorshift::XorShiftRng;
use rand::SeedableRng;
use map_model::{BuildingType};


//distribute residents to buildings w/ geneva stats
pub fn distribute_residents(map: &mut Map){
    let geneva_sub_regions = get_geneva_sub_area(&map);
    let mut rng = XorShiftRng::seed_from_u64(1312);
    for sub_region in geneva_sub_regions {
        for (home, n) in popdat::distribute_population_to_homes(
            geo::Polygon::from(sub_region.0),
            0,
            &map,
            &mut rng,
        ) {
            let bldg_type = match map.get_b(home).bldg_type {
                BuildingType::Residential {
                    num_housing_units, ..
                } => BuildingType::Residential {
                    num_housing_units,
                    num_residents: n,
                },
                BuildingType::ResidentialCommercial(_, worker_cap) => {
                    BuildingType::ResidentialCommercial(n, worker_cap)
                }
                _ => unreachable!(),
            };
            map.hack_override_bldg_type(home, bldg_type);
        }
    }
    map.save();
}

pub fn get_geneva_sub_area(map: &Map) -> Vec<(geom::Polygon, std::option::Option<serde_json::Map<std::string::String, serde_json::Value>>)> {
    let geojson_str = match fs::read_to_string("data/system/ch/geneva/additional_data/GEO_GIREC_simplified.json") {
        Err(e) => panic!("{:?}", e),
        Ok(s) => s
    };

    let geneva_geojson = match geojson_str.parse::<GeoJson>() {
        Err(e) => panic!("{:?}", e),
        Ok(geojson) => geojson
    };

    let mut out = Vec::new();
    for sub_area in process_geojson(&geneva_geojson, map){
        let pts = sub_area.0.points();
        if pts
            .iter()
            .all(|pt| !map.get_boundary_polygon().contains_pt(*pt))
            {
                continue;
            }
            out.push(sub_area);
    }
    println!("NUMBER OF SUB REGION : {}", out.len());
    out
}

// https://docs.rs/geojson/0.22.2/geojson/
fn process_geojson(gj: &GeoJson, map: &Map) -> Vec<(geom::Polygon, std::option::Option<serde_json::Map<std::string::String, serde_json::Value>>)>{
    let mut out: Vec<(Polygon, Option<serde_json::Map<String, serde_json::Value>>)> = Vec::new();
    match *gj {
        GeoJson::FeatureCollection(ref ctn) => {
            for feature in &ctn.features {
                if let Some(ref geom) = feature.geometry {
                    match match_geometry(geom, map) {
                        Ok(poly) => out.push((poly, feature.properties.clone())),
                        Err(s) => println!("{}", s),
                    }
                }
            }
        }
        _ => println!("Not good file"),
    }
    out
}

// https://docs.rs/geojson/0.22.2/geojson/
/// Process GeoJSON geometries
fn match_geometry(geom: &Geometry, map: &Map) -> Result<Polygon, &'static str>{
    // let mut gps_poly: Polygon;
    match &geom.value {
        Value::Polygon(p) => 
            // match Polygon::from_geojson(&p) {
            //     Ok(poly) => gps_poly = poly.clone(),
            //     // return Ok(poly.clone()),
            //     _ => return Err("could not get polygon from geojson"),
            // }
            return Ok(map_poly_from_json_gps(p, map)),
        // Value::MultiPolygon(_) => println!("Matched a MultiPolygon"),
        // Value::GeometryCollection(ref gc) => {
        //     println!("Matched a GeometryCollection");
        //     // GeometryCollections contain other Geometry types, and can nest
        //     // we deal with this by recursively processing each geometry
        //     for geometry in gc {
        //         match_geometry(geometry)
        //     }
        // }
        // // Point, LineString, and their Multi– counterparts
        _ => return Err("not a polygon"),
    }
    // let mut out_vec: Vec<Pt2D> = Vec::new(); 
    // for p in gps_poly.points() {
    //     out_vec.push(LonLat::new(p.x(), p.y()).to_pt(&map.get_gps_bounds()))
    // }
    // Ok(Polygon::buggy_new(out_vec))
}


pub fn map_poly_from_json_gps(raw: &[Vec<Vec<f64>>], map: &Map) -> Polygon {
    let mut rings = Vec::new();
    for pts in raw {
        let transformed: Vec<Pt2D> =
            pts.iter().map(|pair| LonLat::new(pair[0], pair[1]).to_pt(&map.get_gps_bounds())).collect();
        rings.push(Ring::new(transformed).unwrap()); //cest moche mais cest pour le debug
    }
    Polygon::from_rings(rings).translate(-60.0, 140.0)
}


