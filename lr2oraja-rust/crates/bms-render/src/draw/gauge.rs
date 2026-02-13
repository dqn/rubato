use bms_skin::image_handle::ImageRegion;
use bms_skin::skin_gauge::GaugePartType;
use bms_skin::skin_object::Rect;
use bms_skin::skin_source::image_index;

/// A single gauge node to draw.
#[derive(Debug, Clone)]
pub struct GaugeNodeCommand {
    pub part_type: GaugePartType,
    pub image_region: ImageRegion,
    pub dst_rect: Rect,
}

/// Default gauge threshold: nodes at or above this ratio are "red" (survival zone).
const GAUGE_RED_THRESHOLD: f32 = 0.8;

/// Computes the gauge node draw commands.
///
/// - `nodes`: total number of gauge nodes
/// - `gauge_value`: current gauge value (0.0 to 1.0)
/// - `parts`: the gauge parts (part_type, images, timer, cycle)
/// - `timer_time`: elapsed time for part animation
/// - `dst`: destination rect for the entire gauge
///
/// Returns a list of draw commands, one per node.
pub fn compute_gauge_draw(
    nodes: i32,
    gauge_value: f32,
    parts: &[(GaugePartType, Vec<ImageRegion>, Option<i32>, i32)],
    timer_time: i64,
    dst: &Rect,
) -> Vec<GaugeNodeCommand> {
    if nodes <= 0 {
        return vec![];
    }

    let nodes = nodes as usize;
    let node_w = dst.w / nodes as f32;
    let filled_nodes = (gauge_value.clamp(0.0, 1.0) * nodes as f32).ceil() as usize;

    let mut commands = Vec::with_capacity(nodes);

    for i in 0..nodes {
        let is_filled = i < filled_nodes;
        let is_red_zone = (i as f32 / nodes as f32) >= GAUGE_RED_THRESHOLD;

        let target_type = if is_filled {
            if is_red_zone {
                GaugePartType::FrontRed
            } else {
                GaugePartType::FrontGreen
            }
        } else if is_red_zone {
            GaugePartType::BackRed
        } else {
            GaugePartType::BackGreen
        };

        // Find matching part
        let part = parts.iter().find(|(pt, _, _, _)| *pt == target_type);

        if let Some((_part_type, images, _timer, cycle)) = part
            && !images.is_empty()
        {
            let idx = image_index(images.len(), timer_time, *cycle);
            commands.push(GaugeNodeCommand {
                part_type: target_type,
                image_region: images[idx],
                dst_rect: Rect::new(dst.x + node_w * i as f32, dst.y, node_w, dst.h),
            });
        }
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_skin::image_handle::ImageHandle;

    fn make_region(id: u32) -> ImageRegion {
        ImageRegion {
            handle: ImageHandle(id),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }

    fn make_parts() -> Vec<(GaugePartType, Vec<ImageRegion>, Option<i32>, i32)> {
        vec![
            (GaugePartType::FrontGreen, vec![make_region(1)], None, 0),
            (GaugePartType::FrontRed, vec![make_region(2)], None, 0),
            (GaugePartType::BackGreen, vec![make_region(3)], None, 0),
            (GaugePartType::BackRed, vec![make_region(4)], None, 0),
        ]
    }

    #[test]
    fn full_gauge() {
        let dst = Rect::new(0.0, 0.0, 500.0, 20.0);
        let cmds = compute_gauge_draw(50, 1.0, &make_parts(), 0, &dst);
        assert_eq!(cmds.len(), 50);
        // All nodes should be front (filled)
        for cmd in &cmds {
            assert!(
                cmd.part_type == GaugePartType::FrontGreen
                    || cmd.part_type == GaugePartType::FrontRed
            );
        }
    }

    #[test]
    fn empty_gauge() {
        let dst = Rect::new(0.0, 0.0, 500.0, 20.0);
        let cmds = compute_gauge_draw(50, 0.0, &make_parts(), 0, &dst);
        assert_eq!(cmds.len(), 50);
        // All nodes should be back (unfilled)
        for cmd in &cmds {
            assert!(
                cmd.part_type == GaugePartType::BackGreen
                    || cmd.part_type == GaugePartType::BackRed
            );
        }
    }

    #[test]
    fn half_gauge() {
        let dst = Rect::new(0.0, 0.0, 100.0, 20.0);
        let cmds = compute_gauge_draw(10, 0.5, &make_parts(), 0, &dst);
        assert_eq!(cmds.len(), 10);
        // First 5 filled, last 5 unfilled
        let filled = cmds
            .iter()
            .filter(|c| {
                c.part_type == GaugePartType::FrontGreen || c.part_type == GaugePartType::FrontRed
            })
            .count();
        assert_eq!(filled, 5);
    }

    #[test]
    fn correct_node_count() {
        let dst = Rect::new(0.0, 0.0, 250.0, 20.0);
        let cmds = compute_gauge_draw(25, 0.5, &make_parts(), 0, &dst);
        assert_eq!(cmds.len(), 25);
        let node_w = 250.0 / 25.0;
        // Verify each node has correct width
        for cmd in &cmds {
            assert!((cmd.dst_rect.w - node_w).abs() < 0.001);
        }
    }

    #[test]
    fn red_zone_threshold() {
        let dst = Rect::new(0.0, 0.0, 100.0, 20.0);
        // 10 nodes, threshold at 0.8 -> nodes 8,9 are red zone
        let cmds = compute_gauge_draw(10, 1.0, &make_parts(), 0, &dst);
        // Nodes 0-7: FrontGreen, Nodes 8-9: FrontRed
        for (i, cmd) in cmds.iter().enumerate() {
            if i < 8 {
                assert_eq!(cmd.part_type, GaugePartType::FrontGreen);
            } else {
                assert_eq!(cmd.part_type, GaugePartType::FrontRed);
            }
        }
    }
}
