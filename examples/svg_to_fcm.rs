//! Complete SVG to Print-and-Cut FCM conversion example
//!
//! Run with: cargo run --example svg_to_fcm

use fcmlib::{
    svg_path::{SvgConfig, SvgPathParser},
    AlignmentData, CutData, FcmFile, FileHeader, FileType, FileVariant,
    Generator, Path, PathTool, Piece, PieceRestrictions, PieceTable, Point,
};

/// Standard page sizes in mm
struct PageSize {
    width_mm: f64,
    height_mm: f64,
}

impl PageSize {
    const LETTER: PageSize = PageSize { width_mm: 215.9, height_mm: 279.4 };
    const A4: PageSize = PageSize { width_mm: 210.0, height_mm: 297.0 };
}

/// Calculate registration mark positions for a given page size
fn calculate_registration_marks(page: &PageSize) -> Vec<Point> {
    // Standard Brother insets (approximately 12mm from edges)
    let x_inset = 1200i32;  // 12.00mm in hundredths
    let y_inset = 1398i32;  // 13.98mm in hundredths

    let page_width = (page.width_mm * 100.0) as i32;
    let page_height = (page.height_mm * 100.0) as i32;

    vec![
        Point { x: x_inset, y: y_inset },                              // Top-left
        Point { x: page_width - x_inset, y: y_inset },                 // Top-right
        Point { x: page_width - x_inset, y: page_height - y_inset },   // Bottom-right
        Point { x: x_inset, y: page_height - y_inset },                // Bottom-left
    ]
}

/// Convert SVG path data to a print-and-cut FCM file
fn svg_to_print_and_cut_fcm(
    svg_path_d: &str,
    page: &PageSize,
    svg_dpi: f64,
) -> Result<FcmFile, Box<dyn std::error::Error>> {
    // Parse SVG path
    let config = SvgConfig {
        dpi: svg_dpi,
        scale: 1.0,
        offset_x_mm: 0.0,
        offset_y_mm: 0.0,
    };

    let parser = SvgPathParser::new(config);
    let shapes = parser.parse(svg_path_d)?;

    // Calculate bounding box for piece dimensions
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for shape in &shapes {
        min_x = min_x.min(shape.start.x);
        min_y = min_y.min(shape.start.y);
        max_x = max_x.max(shape.start.x);
        max_y = max_y.max(shape.start.y);

        for outline in &shape.outlines {
            match outline {
                fcmlib::Outline::Line(segments) => {
                    for seg in segments {
                        min_x = min_x.min(seg.end.x);
                        min_y = min_y.min(seg.end.y);
                        max_x = max_x.max(seg.end.x);
                        max_y = max_y.max(seg.end.y);
                    }
                }
                fcmlib::Outline::Bezier(segments) => {
                    for seg in segments {
                        min_x = min_x.min(seg.end.x).min(seg.control1.x).min(seg.control2.x);
                        min_y = min_y.min(seg.end.y).min(seg.control1.y).min(seg.control2.y);
                        max_x = max_x.max(seg.end.x).max(seg.control1.x).max(seg.control2.x);
                        max_y = max_y.max(seg.end.y).max(seg.control1.y).max(seg.control2.y);
                    }
                }
            }
        }
    }

    let width = (max_x - min_x) as u32;
    let height = (max_y - min_y) as u32;
    let center_x = (min_x + max_x) as f32 / 2.0;
    let center_y = (min_y + max_y) as f32 / 2.0;

    // Create paths from shapes
    let paths: Vec<Path> = shapes
        .into_iter()
        .map(|shape| Path {
            tool: PathTool::TOOL_CUT | PathTool::TOOL_DRAW,
            shape: Some(fcmlib::PathShape {
                start: Point {
                    x: shape.start.x - center_x as i32,
                    y: shape.start.y - center_y as i32,
                },
                outlines: shape.outlines.into_iter().map(|outline| {
                    match outline {
                        fcmlib::Outline::Line(segments) => {
                            fcmlib::Outline::Line(segments.into_iter().map(|seg| {
                                fcmlib::SegmentLine {
                                    end: Point {
                                        x: seg.end.x - center_x as i32,
                                        y: seg.end.y - center_y as i32,
                                    }
                                }
                            }).collect())
                        }
                        fcmlib::Outline::Bezier(segments) => {
                            fcmlib::Outline::Bezier(segments.into_iter().map(|seg| {
                                fcmlib::SegmentBezier {
                                    control1: Point {
                                        x: seg.control1.x - center_x as i32,
                                        y: seg.control1.y - center_y as i32,
                                    },
                                    control2: Point {
                                        x: seg.control2.x - center_x as i32,
                                        y: seg.control2.y - center_y as i32,
                                    },
                                    end: Point {
                                        x: seg.end.x - center_x as i32,
                                        y: seg.end.y - center_y as i32,
                                    },
                                }
                            }).collect())
                        }
                    }
                }).collect(),
            }),
            rhinestone_diameter: None,
            rhinestones: vec![],
        })
        .collect();

    // Create piece
    let piece = Piece {
        width,
        height,
        transform: Some((1.0, 0.0, 0.0, 1.0, center_x, center_y)),
        expansion_limit_value: 0,
        reduction_limit_value: 0,
        restriction_flags: PieceRestrictions::empty(),
        label: String::new(),
        paths,
    };

    // Page dimensions in FCM units
    let page_width = (page.width_mm * 100.0) as u32;
    let page_height = (page.height_mm * 100.0) as u32;

    // Create FCM file
    Ok(FcmFile {
        file_header: FileHeader {
            variant: FileVariant::VCM,
            version: String::from("0100"),
            content_id: 400000002,
            short_name: String::new(),
            long_name: String::from(" "),
            author_name: String::from(" "),
            copyright: String::new(),
            thumbnail_block_size_width: 3,
            thumbnail_block_size_height: 3,
            thumbnail: vec![0; 9],
            generator: Generator::App(1),
            print_to_cut: Some(true),
        },
        cut_data: CutData {
            file_type: FileType::PrintAndCut,
            mat_id: 0,
            cut_width: page_width,
            cut_height: page_height,
            seam_allowance_width: 2000,
            alignment: Some(AlignmentData {
                needed: true,
                marks: calculate_registration_marks(page),
            }),
        },
        piece_table: PieceTable {
            pieces: vec![(0, piece)],
        },
    })
}

fn main() {
    // Example: A star shape centered at 100,100 with 50px radius
    let star_path = "M 100,50 L 112,85 L 150,85 L 120,105 L 132,140 L 100,120 L 68,140 L 80,105 L 50,85 L 88,85 Z";

    // Example: A simple rectangle
    let rect_path = "M 200,200 L 400,200 L 400,300 L 200,300 Z";

    // Example: A heart shape with bezier curves
    let heart_path = "M 300,250 C 300,200 250,200 250,250 C 250,280 300,320 300,320 C 300,320 350,280 350,250 C 350,200 300,200 300,250 Z";

    println!("=== SVG to Print-and-Cut FCM Converter ===\n");

    // Convert rectangle (simplest case)
    println!("Converting rectangle...");
    match svg_to_print_and_cut_fcm(rect_path, &PageSize::LETTER, 96.0) {
        Ok(fcm) => {
            let output = "rectangle_print_and_cut.fcm";
            fcm.to_file(output).expect("Failed to write FCM");
            println!("  Created: {}", output);
            println!("  Page: {}mm x {}mm",
                fcm.cut_data.cut_width as f64 / 100.0,
                fcm.cut_data.cut_height as f64 / 100.0);
            if let Some(align) = &fcm.cut_data.alignment {
                println!("  Registration marks: {}", align.marks.len());
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Convert star
    println!("\nConverting star...");
    match svg_to_print_and_cut_fcm(star_path, &PageSize::LETTER, 96.0) {
        Ok(fcm) => {
            let output = "star_print_and_cut.fcm";
            fcm.to_file(output).expect("Failed to write FCM");
            println!("  Created: {}", output);
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Convert heart (with bezier curves)
    println!("\nConverting heart (bezier curves)...");
    match svg_to_print_and_cut_fcm(heart_path, &PageSize::LETTER, 96.0) {
        Ok(fcm) => {
            let output = "heart_print_and_cut.fcm";
            fcm.to_file(output).expect("Failed to write FCM");
            println!("  Created: {}", output);

            // Show piece info
            if let Some((_, piece)) = fcm.piece_table.pieces.first() {
                println!("  Shape size: {:.2}mm x {:.2}mm",
                    piece.width as f64 / 100.0,
                    piece.height as f64 / 100.0);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!("\n=== Done! ===");
    println!("\nFiles created can be loaded on a Brother ScanNCut.");
    println!("Remember to print the artwork with registration marks first!");
}
