use std::fs;

use geom::LonLat;
use geom::Polygon;
use geojson::{GeoJson, Geometry, Value};
use geom::Distance;
use geom::Pt2D;
use geom::Ring;
use map_model::Map;
use widgetry::GfxCtx;

pub fn show_geneva_sub_area(g: &mut GfxCtx, map: &Map){
    let test_color = widgetry::Color::RED;
    for p in read_geneva_sub_area(map) {
        // println!("{}", p);
        g.draw_polygon(test_color, p.to_outline(Distance::meters(5.0)).unwrap());
    }
    // let b_id: BuildingID = BuildingID {0: 3354 };
    // println!("{}", map.get_b(b_id).polygon.clone());

    // let mut test_vecpt: Vec<Pt2D> = Vec::new();
    // // test_vecpt.push(Pt2D::new(2487458.7863857746, 1118655.239805065));
    // // test_vecpt.push(Pt2D::new(2487942.715158604, 1118538.0861771032));
    // // test_vecpt.push(Pt2D::new(2488007.551915489, 1117357.053375803));
    // test_vecpt.push(Pt2D::new(0.6557, 1713.1572));
    // test_vecpt.push(Pt2D::new(5451.792, 1726.4784));
    // test_vecpt.push(Pt2D::new(5351.792, 1626.4784));
    // test_vecpt.push(Pt2D::new(0.6557, 1713.1572));

    // let test_polygon = Polygon::buggy_new(test_vecpt);
}

pub fn read_geneva_sub_area(map: &Map) -> Vec<Polygon> {
    let geojson_str = match fs::read_to_string("data/system/ch/geneva/additional_data/GEO_GIREC_simplified.json") {
        Err(e) => panic!("{:?}", e),
        Ok(s) => s
    };

    let geneva_geojson = match geojson_str.parse::<GeoJson>() {
        Err(e) => panic!("{:?}", e),
        Ok(geojson) => geojson
    };
    let mut out : Vec<Polygon> = Vec::new();
    for sub_area in process_geojson(&geneva_geojson, map){
        let pts = sub_area.points();
        if pts
            .iter()
            .all(|pt| !map.get_boundary_polygon().contains_pt(*pt))
            {
                continue;
            }
            out.push(sub_area);
    }
    out
}

// https://docs.rs/geojson/0.22.2/geojson/
fn process_geojson(gj: &GeoJson, map: &Map) -> Vec<Polygon>{
    let mut out: Vec<Polygon> = Vec::new();
    match *gj {
        GeoJson::FeatureCollection(ref ctn) => {
            for feature in &ctn.features {
                if let Some(ref geom) = feature.geometry {
                    match match_geometry(geom, map) {
                        Ok(poly) => out.push(poly),
                        Err(s) => println!("{}", s),
                    }
                }
            }
        }
        GeoJson::Feature(ref feature) => {
            if let Some(ref geom) = feature.geometry {
                match match_geometry(geom, map) {
                    Ok(poly) => out.push(poly),
                    Err(s) => println!("{}", s),
                }
            }
        }
        GeoJson::Geometry(ref geometry) => 
            match match_geometry(geometry, map) {
                Ok(poly) => out.push(poly),
                Err(s) => println!("{}", s),
            },
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


