use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

use beatoraja_pattern::lane_shuffle_modifier::{
    LaneMirrorShuffleModifier, LaneRandomShuffleModifier, LaneRotateShuffleModifier,
};
use beatoraja_pattern::pattern_modifier::PatternModifier;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

fn make_test_model(mode: &Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_all_time_line(timelines);
    model.set_mode(mode.clone());
    model
}

/// Build a model with the given number of timelines, each having notes on
/// all lanes. Simulates a dense BMS chart.
fn make_dense_model(mode: &Mode, timeline_count: usize) -> BMSModel {
    let key_count = mode.key() as usize;
    let mut timelines = Vec::with_capacity(timeline_count);
    for i in 0..timeline_count {
        let mut tl = TimeLine::new(i as f64, (i * 1000) as i64, key_count as i32);
        for lane in 0..key_count {
            let wav = (i as i32) * 100 + lane as i32;
            tl.set_note(lane as i32, Some(Note::new_normal(wav)));
        }
        timelines.push(tl);
    }
    make_test_model(mode, timelines)
}

fn bench_mirror(c: &mut Criterion) {
    let mode = Mode::BEAT_7K;
    c.bench_function("mirror", |b| {
        b.iter_batched(
            || {
                let model = make_dense_model(&mode, 100);
                let mut modifier = LaneMirrorShuffleModifier::new(0, false);
                modifier.set_seed(42);
                (model, modifier)
            },
            |(mut model, mut modifier)| {
                modifier.modify(black_box(&mut model));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_random(c: &mut Criterion) {
    let mode = Mode::BEAT_7K;
    c.bench_function("random", |b| {
        b.iter_batched(
            || {
                let model = make_dense_model(&mode, 100);
                let mut modifier = LaneRandomShuffleModifier::new(0, false);
                modifier.set_seed(42);
                (model, modifier)
            },
            |(mut model, mut modifier)| {
                modifier.modify(black_box(&mut model));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_rotate(c: &mut Criterion) {
    let mode = Mode::BEAT_7K;
    c.bench_function("rotate", |b| {
        b.iter_batched(
            || {
                let model = make_dense_model(&mode, 100);
                let mut modifier = LaneRotateShuffleModifier::new(0, false);
                modifier.set_seed(42);
                (model, modifier)
            },
            |(mut model, mut modifier)| {
                modifier.modify(black_box(&mut model));
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_mirror, bench_random, bench_rotate);
criterion_main!(benches);
