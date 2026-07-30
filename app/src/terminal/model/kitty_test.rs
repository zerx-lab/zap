use base64::engine::general_purpose::STANDARD as BASE64;
use warp_core::features::FeatureFlag;

use crate::terminal::model::image_map::StoredImageMetadata;
use crate::terminal::model::index::Point;
use crate::terminal::model::kitty::MAX_ANIMATION_FRAMES;
use crate::terminal::model::TerminalModel;

/// Builds a single-chunk kitty graphics APC message.
fn kitty_apc(control_data: &str, payload: &[u8]) -> String {
    format!(
        "\x1b_G{};{}\x1b\\",
        control_data,
        base64::Engine::encode(&BASE64, payload)
    )
}

/// A one pixel, 24-bit RGB image, which is the smallest payload that passes
/// kitty's RGB size validation.
fn one_pixel_rgb() -> &'static [u8] {
    &[0xff, 0x00, 0x00]
}

/// A terminal with a running command, so that graphics land in a block's output
/// grid. Blocks that haven't started executing route to their header grid, which
/// doesn't handle kitty actions at all.
fn kitty_terminal() -> TerminalModel {
    let mut terminal = TerminalModel::mock(None, None);
    terminal.simulate_cmd("kitty");
    terminal
}

/// Where the cursor sits in the block that graphics are landing in.
fn cursor_point(terminal: &TerminalModel) -> Point {
    terminal
        .block_list()
        .active_block()
        .grid_handler()
        .cursor_point()
}

/// The virtual (`U=1`) placements recorded for an image.
fn virtual_placement_ids(terminal: &TerminalModel, image_id: u32) -> Vec<u32> {
    let Some(StoredImageMetadata::Kitty(metadata)) = terminal.image_id_to_metadata.get(&image_id)
    else {
        return Vec::new();
    };

    let mut ids: Vec<u32> = metadata.virtual_placements.keys().copied().collect();
    ids.sort_unstable();
    ids
}

fn reply_for(control_data: &str, payload: &[u8]) -> String {
    let mut terminal = kitty_terminal();
    let written = terminal.process_bytes_capturing(kitty_apc(control_data, payload).as_str());
    String::from_utf8_lossy(&written).into_owned()
}

/// Transmits and displays a one cell image at the cursor without moving the
/// cursor afterwards (`C=1`), so that tests control which cell it lands on.
fn place_image(terminal: &mut TerminalModel, control_data: &str) {
    terminal.process_bytes(
        kitty_apc(
            &format!("a=T,f=24,s=1,v=1,C=1,{control_data}"),
            one_pixel_rgb(),
        )
        .as_str(),
    );
}

/// A terminal holding a stored one pixel image with id 1, which animation
/// messages can then address.
fn terminal_with_stored_image() -> TerminalModel {
    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=t,i=1,f=24,s=1,v=1", one_pixel_rgb()).as_str());
    terminal
}

/// Sends an animation message and returns whatever was replied to the shell.
fn animate(terminal: &mut TerminalModel, control_data: &str, payload: &[u8]) -> String {
    let written = terminal.process_bytes_capturing(kitty_apc(control_data, payload).as_str());
    String::from_utf8_lossy(&written).into_owned()
}

/// The gaps of the animation frames recorded for an image, in playback order.
fn frame_gaps(terminal: &TerminalModel, image_id: u32) -> Vec<u32> {
    let Some(StoredImageMetadata::Kitty(metadata)) = terminal.image_id_to_metadata.get(&image_id)
    else {
        return Vec::new();
    };

    metadata.frames.iter().map(|&(_, gap)| gap).collect()
}

/// Whether an image's animation is running.
fn is_playing(terminal: &TerminalModel, image_id: u32) -> bool {
    matches!(
        terminal.image_id_to_metadata.get(&image_id),
        Some(StoredImageMetadata::Kitty(metadata)) if metadata.playing
    )
}

/// Sends a delete message and returns whatever was replied to the shell.
fn delete(terminal: &mut TerminalModel, control_data: &str) -> String {
    let written =
        terminal.process_bytes_capturing(kitty_apc(&format!("a=d,{control_data}"), &[]).as_str());
    String::from_utf8_lossy(&written).into_owned()
}

/// Whether the active block's grid still holds the given placement.
fn has_placement(terminal: &TerminalModel, image_id: u32, placement_id: u32) -> bool {
    terminal
        .block_list()
        .active_block()
        .grid_handler()
        .get_image_placement_data(image_id, placement_id)
        .is_some()
}

/// A terminal holding two one cell placements: image 1 at the cell kitty calls
/// `x=1,y=1` and image 2 at `x=3,y=2`. The cursor is left on image 2's cell.
fn terminal_with_two_placements() -> TerminalModel {
    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1");
    terminal.process_bytes("\r\n  ");
    place_image(&mut terminal, "i=2,p=2");
    terminal
}

#[test]
fn zero_size_transmit_and_display_does_not_panic() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let reply = reply_for("a=T,i=1,f=24,s=0,v=0", &[]);

    // The action is a no-op, but it must still be acknowledged rather than panic.
    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
}

#[test]
fn zero_size_display_of_stored_image_does_not_panic() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=t,i=1,f=24,s=0,v=0", &[]).as_str());
    let written = terminal.process_bytes_capturing(kitty_apc("a=p,i=1", &[]).as_str());

    let reply = String::from_utf8_lossy(&written);
    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
}

#[test]
fn query_reply_is_sent_despite_quiet_mode() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let reply = reply_for("a=q,i=1,q=1,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
}

#[test]
fn unknown_image_id_error_reply_uses_enoent() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let reply = reply_for("a=p,i=999", &[]);

    assert!(
        reply.contains("i=999;ENOENT:"),
        "unexpected reply: {reply:?}"
    );
}

#[test]
fn ok_reply_echoes_image_and_placement_ids() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let reply = reply_for("a=T,i=7,p=3,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains("i=7,p=3;OK"), "unexpected reply: {reply:?}");
}

#[test]
fn quiet_mode_one_suppresses_ok_but_not_errors() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let ok_reply = reply_for("a=T,i=1,q=1,f=24,s=1,v=1", one_pixel_rgb());
    assert!(ok_reply.is_empty(), "unexpected reply: {ok_reply:?}");

    let error_reply = reply_for("a=p,i=999,q=1", &[]);
    assert!(
        error_reply.contains("ENOENT:"),
        "unexpected reply: {error_reply:?}"
    );
}

#[test]
fn quiet_mode_two_suppresses_errors() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let error_reply = reply_for("a=p,i=999,q=2", &[]);
    assert!(error_reply.is_empty(), "unexpected reply: {error_reply:?}");

    let ok_reply = reply_for("a=T,i=1,q=2,f=24,s=1,v=1", one_pixel_rgb());
    assert!(ok_reply.is_empty(), "unexpected reply: {ok_reply:?}");
}

#[test]
fn delete_all_removes_every_placement() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_two_placements();
    delete(&mut terminal, "d=a");

    assert!(!has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 2, 2));
}

#[test]
fn delete_by_id_removes_only_that_image() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_two_placements();
    // A second placement of image 1, so we can tell "every placement of an
    // image" apart from "one placement".
    place_image(&mut terminal, "i=1,p=9");

    delete(&mut terminal, "d=i,i=1");

    assert!(!has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 1, 9));
    assert!(has_placement(&terminal, 2, 2));
}

#[test]
fn delete_by_id_with_placement_id_removes_only_that_placement() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_two_placements();
    place_image(&mut terminal, "i=1,p=9");

    delete(&mut terminal, "d=i,i=1,p=9");

    assert!(has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 1, 9));
    assert!(has_placement(&terminal, 2, 2));
}

#[test]
fn delete_by_number_removes_the_numbered_image() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    // `i=` still identifies the image; `I=` is the client's own number, which is
    // what `d=n` selects on.
    place_image(&mut terminal, "i=5,p=5,I=7");
    place_image(&mut terminal, "i=6,p=6");

    delete(&mut terminal, "d=n,I=7");

    assert!(!has_placement(&terminal, 5, 5));
    assert!(has_placement(&terminal, 6, 6));
}

#[test]
fn delete_at_cursor_removes_the_placement_under_the_cursor() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    // The helper leaves the cursor on image 2's cell.
    let mut terminal = terminal_with_two_placements();
    delete(&mut terminal, "d=c");

    assert!(has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 2, 2));
}

#[test]
fn delete_at_point_removes_the_placement_in_that_cell() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_two_placements();
    delete(&mut terminal, "d=p,x=1,y=1");

    assert!(!has_placement(&terminal, 1, 1));
    assert!(has_placement(&terminal, 2, 2));
}

#[test]
fn delete_at_point_with_z_index_only_removes_matching_depths() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    // Two placements stacked on the same cell at different depths.
    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1,z=5");
    place_image(&mut terminal, "i=2,p=2,z=6");

    delete(&mut terminal, "d=q,x=1,y=1,z=6");

    assert!(has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 2, 2));
}

#[test]
fn delete_in_column_removes_placements_in_that_column() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_two_placements();
    delete(&mut terminal, "d=x,x=3");

    assert!(has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 2, 2));
}

#[test]
fn delete_in_row_removes_placements_in_that_row() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_two_placements();
    delete(&mut terminal, "d=y,y=2");

    assert!(has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 2, 2));
}

#[test]
fn delete_by_z_index_removes_placements_at_that_depth() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1,z=5");
    place_image(&mut terminal, "i=2,p=2");

    delete(&mut terminal, "d=z,z=5");

    assert!(!has_placement(&terminal, 1, 1));
    assert!(has_placement(&terminal, 2, 2));
}

#[test]
fn delete_by_id_range_removes_ids_within_the_range() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    for image_id in 1..=4 {
        place_image(&mut terminal, &format!("i={image_id},p={image_id}"));
    }

    delete(&mut terminal, "d=r,x=2,y=3");

    assert!(has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 2, 2));
    assert!(!has_placement(&terminal, 3, 3));
    assert!(has_placement(&terminal, 4, 4));
}

#[test]
fn unknown_delete_specifier_is_rejected_and_deletes_nothing() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    // Regression test: an unrecognized `d=` used to fall through to "delete all".
    for specifier in ["d=w", "d=!"] {
        let mut terminal = terminal_with_two_placements();
        let reply = delete(&mut terminal, &format!("{specifier},i=1"));

        assert!(
            reply.contains("i=1;EINVAL:"),
            "unexpected reply for {specifier}: {reply:?}"
        );
        assert!(
            has_placement(&terminal, 1, 1),
            "{specifier} deleted image 1"
        );
        assert!(
            has_placement(&terminal, 2, 2),
            "{specifier} deleted image 2"
        );
    }
}

#[test]
fn uppercase_delete_also_frees_the_image_data() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1");
    delete(&mut terminal, "d=I,i=1");

    // The image data is gone, so it can no longer be placed again.
    let reply = String::from_utf8_lossy(
        &terminal.process_bytes_capturing(kitty_apc("a=p,i=1", &[]).as_str()),
    )
    .into_owned();
    assert!(reply.contains("i=1;ENOENT:"), "unexpected reply: {reply:?}");

    // The lowercase form only drops placements, so the image survives.
    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1");
    delete(&mut terminal, "d=i,i=1");

    let reply = String::from_utf8_lossy(
        &terminal.process_bytes_capturing(kitty_apc("a=p,i=1", &[]).as_str()),
    )
    .into_owned();
    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
}

#[test]
fn uppercase_positional_delete_also_frees_the_image_data() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1");
    delete(&mut terminal, "d=P,x=1,y=1");

    let reply = String::from_utf8_lossy(
        &terminal.process_bytes_capturing(kitty_apc("a=p,i=1", &[]).as_str()),
    )
    .into_owned();
    assert!(reply.contains("i=1;ENOENT:"), "unexpected reply: {reply:?}");
}

#[test]
fn delete_frames_is_reported_as_unsupported() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1");

    let reply = delete(&mut terminal, "d=f,i=1");

    assert!(
        reply.contains("i=1;ENOTSUPP:"),
        "unexpected reply: {reply:?}"
    );
    // Animation frames are not stored yet, so nothing may be removed.
    assert!(has_placement(&terminal, 1, 1));
}

#[test]
fn unicode_placeholder_transmit_and_display_is_accepted() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let reply = reply_for("a=T,U=1,i=1,p=2,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains("i=1,p=2;OK"), "unexpected reply: {reply:?}");
    assert!(!reply.contains("EINVAL"), "unexpected reply: {reply:?}");
}

#[test]
fn unicode_placeholder_display_of_stored_image_is_accepted() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=t,i=1,f=24,s=1,v=1", one_pixel_rgb()).as_str());
    let written = terminal.process_bytes_capturing(kitty_apc("a=p,U=1,i=1,p=5", &[]).as_str());

    let reply = String::from_utf8_lossy(&written);
    assert!(reply.contains("i=1,p=5;OK"), "unexpected reply: {reply:?}");
    assert_eq!(virtual_placement_ids(&terminal, 1), vec![5]);
}

#[test]
fn unicode_placeholder_placement_does_not_move_the_cursor() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    let before = cursor_point(&terminal);

    // A tall image: an anchored placement would scroll in rows of whitespace
    // and leave the cursor past the image.
    terminal.process_bytes(kitty_apc("a=T,U=1,i=1,r=4,c=4,f=24,s=1,v=1", one_pixel_rgb()).as_str());

    assert_eq!(
        cursor_point(&terminal),
        before,
        "a virtual placement must not move the cursor"
    );
}

#[test]
fn unicode_placeholder_records_rows_and_columns_unresolved() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal
        .process_bytes(kitty_apc("a=T,U=1,i=3,p=1,r=2,c=6,f=24,s=1,v=1", one_pixel_rgb()).as_str());

    let Some(StoredImageMetadata::Kitty(metadata)) = terminal.image_id_to_metadata.get(&3) else {
        panic!("image 3 should have kitty metadata");
    };
    let placement = metadata
        .virtual_placements
        .get(&1)
        .expect("placement 1 should be recorded");

    // Left unresolved so that a font size change re-tiles the image.
    assert_eq!(placement.rows, Some(2));
    assert_eq!(placement.cols, Some(6));
}

#[test]
fn anchored_placement_records_no_virtual_placement() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=T,i=1,p=2,f=24,s=1,v=1", one_pixel_rgb()).as_str());

    assert!(virtual_placement_ids(&terminal, 1).is_empty());
}

#[test]
fn extreme_aspect_ratio_display_does_not_underflow() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    // A 4000x1 image squeezed into one column: the desired height truncates to
    // zero cells, which used to compute `0usize - 1` in the newline loop and
    // turn it into an unbounded line-append in release builds.
    let pixels = vec![0u8; 4000 * 3];
    let written = terminal
        .process_bytes_capturing(kitty_apc("a=T,i=7,c=1,f=24,s=4000,v=1", &pixels).as_str());

    let reply = String::from_utf8_lossy(&written);
    assert!(reply.contains("i=7"), "unexpected reply: {reply:?}");
}

#[test]
fn query_is_answered_before_a_command_starts_executing() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    // No `simulate_cmd`: the block is still before `preexec`, so grid-bound
    // actions route to the header grid. A support probe must be answered anyway.
    let mut terminal = TerminalModel::mock(None, None);
    let written = terminal
        .process_bytes_capturing(kitty_apc("a=q,i=31,f=24,s=1,v=1", one_pixel_rgb()).as_str());

    let reply = String::from_utf8_lossy(&written);
    assert!(reply.contains("i=31;OK"), "unexpected reply: {reply:?}");
}

#[test]
fn transmitted_frames_accumulate_with_their_gaps() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();

    for gap in [100, 200, 300] {
        let reply = animate(
            &mut terminal,
            &format!("a=f,i=1,f=24,s=1,v=1,z={gap}"),
            one_pixel_rgb(),
        );
        assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
    }

    // The stored image is frame 1 of the animation, so only the three
    // transmitted frames are recorded.
    assert_eq!(frame_gaps(&terminal, 1), vec![100, 200, 300]);
}

#[test]
fn animation_control_starts_and_stops_playback() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();
    animate(&mut terminal, "a=f,i=1,f=24,s=1,v=1,z=100", one_pixel_rgb());

    // Frames are transmitted to be played, so the first one starts the animation.
    assert!(is_playing(&terminal, 1));

    let reply = animate(&mut terminal, "a=a,i=1,s=1", &[]);
    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
    assert!(!is_playing(&terminal, 1));

    animate(&mut terminal, "a=a,i=1,s=2", &[]);
    assert!(is_playing(&terminal, 1));
}

#[test]
fn animation_control_edits_a_frame_gap() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();
    animate(&mut terminal, "a=f,i=1,f=24,s=1,v=1,z=100", one_pixel_rgb());
    animate(&mut terminal, "a=f,i=1,f=24,s=1,v=1,z=200", one_pixel_rgb());

    // Frame 2 is the first transmitted frame; frame 1 is the stored image.
    let reply = animate(&mut terminal, "a=a,i=1,r=2,z=500", &[]);

    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
    assert_eq!(frame_gaps(&terminal, 1), vec![500, 200]);
}

#[test]
fn frames_needing_compositing_are_unsupported() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();

    // A frame placed at an offset, a frame built on top of another frame, and a
    // frame smaller than the canvas all have to be composited.
    for control_data in [
        "a=f,i=1,f=24,s=1,v=1,x=4",
        "a=f,i=1,f=24,s=1,v=1,y=4",
        "a=f,i=1,f=24,s=1,v=1,c=1",
    ] {
        let reply = animate(&mut terminal, control_data, one_pixel_rgb());
        assert!(
            reply.contains("i=1;ENOTSUPP:"),
            "unexpected reply to {control_data:?}: {reply:?}"
        );
    }

    let reply = animate(
        &mut terminal,
        "a=f,i=1,f=24,s=2,v=1",
        &[0xff, 0x00, 0x00, 0x00, 0xff, 0x00],
    );
    assert!(
        reply.contains("i=1;ENOTSUPP:"),
        "unexpected reply: {reply:?}"
    );

    assert!(frame_gaps(&terminal, 1).is_empty());
}

#[test]
fn compose_action_is_unsupported() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let reply = reply_for("a=c,i=1", &[]);

    assert!(
        reply.contains("i=1;ENOTSUPP:"),
        "unexpected reply: {reply:?}"
    );
}

#[test]
fn zero_image_number_gets_no_reply() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    // `I=0` is the unset default of client libraries that always emit the key;
    // replying would hand them an id they never asked about.
    let reply = reply_for("a=T,I=0,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.is_empty(), "unexpected reply: {reply:?}");
}

#[test]
fn animation_messages_resolve_the_client_image_number() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=t,i=1,I=5,f=24,s=1,v=1", one_pixel_rgb()).as_str());

    let reply = animate(&mut terminal, "a=f,I=5,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains("i=1,I=5;OK"), "unexpected reply: {reply:?}");
    assert_eq!(frame_gaps(&terminal, 1).len(), 1);
}

#[test]
fn frame_edit_of_a_missing_frame_is_an_error_not_an_append() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();

    let reply = animate(&mut terminal, "a=f,i=1,r=7,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains("i=1;ENOENT:"), "unexpected reply: {reply:?}");
    assert!(frame_gaps(&terminal, 1).is_empty());
}

#[test]
fn explicit_zero_base_frame_is_accepted() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();

    // `c=0` means "no base frame": nothing to composite, so nothing to reject.
    let reply = animate(&mut terminal, "a=f,i=1,c=0,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains("i=1;OK"), "unexpected reply: {reply:?}");
    assert_eq!(frame_gaps(&terminal, 1).len(), 1);
}

#[test]
fn animation_frames_are_capped() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = terminal_with_stored_image();
    for _ in 0..MAX_ANIMATION_FRAMES {
        terminal.process_bytes(kitty_apc("a=f,i=1,q=1,f=24,s=1,v=1", one_pixel_rgb()).as_str());
    }
    assert_eq!(frame_gaps(&terminal, 1).len(), MAX_ANIMATION_FRAMES);

    let reply = animate(&mut terminal, "a=f,i=1,f=24,s=1,v=1", one_pixel_rgb());

    assert!(
        reply.contains("i=1;ENOTSUPP:"),
        "unexpected reply: {reply:?}"
    );
    assert_eq!(frame_gaps(&terminal, 1).len(), MAX_ANIMATION_FRAMES);
}

#[test]
fn reply_echoes_the_client_image_number() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    // No `i=`: the reply is how the client learns the id the terminal assigned
    // to its number.
    let reply = reply_for("a=T,I=9,f=24,s=1,v=1", one_pixel_rgb());

    assert!(reply.contains(",I=9;OK"), "unexpected reply: {reply:?}");
    assert!(reply.contains("i="), "unexpected reply: {reply:?}");
}

#[test]
fn delete_all_clears_virtual_placements_without_freeing_the_image() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=T,U=1,i=1,p=5,f=24,s=1,v=1", one_pixel_rgb()).as_str());

    delete(&mut terminal, "d=a");

    assert!(virtual_placement_ids(&terminal, 1).is_empty());
    // Lowercase `d=a` removes placements, not the stored image data.
    assert!(terminal.image_id_to_metadata.contains_key(&1));
}

#[test]
fn delete_by_id_clears_virtual_placements() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=T,U=1,i=1,p=5,f=24,s=1,v=1", one_pixel_rgb()).as_str());
    terminal.process_bytes(kitty_apc("a=p,U=1,i=1,p=6", &[]).as_str());

    delete(&mut terminal, "d=i,i=1,p=5");
    assert_eq!(virtual_placement_ids(&terminal, 1), vec![6]);

    delete(&mut terminal, "d=i,i=1");
    assert!(virtual_placement_ids(&terminal, 1).is_empty());
}

#[test]
fn delete_by_z_index_clears_matching_virtual_placements() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    terminal.process_bytes(kitty_apc("a=T,U=1,i=1,p=5,z=3,f=24,s=1,v=1", one_pixel_rgb()).as_str());
    terminal.process_bytes(kitty_apc("a=p,U=1,i=1,p=6,z=4", &[]).as_str());

    delete(&mut terminal, "d=z,z=3");

    assert_eq!(virtual_placement_ids(&terminal, 1), vec![6]);
}

#[test]
fn uppercase_z_delete_evicts_every_placement_of_a_freed_image() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1,z=5");
    place_image(&mut terminal, "i=1,p=2,z=6");

    // Freeing image 1 by z-index must not leave the z=6 placement behind as a
    // blank hole.
    delete(&mut terminal, "d=Z,z=5");

    assert!(!has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 1, 2));
    assert!(!terminal.image_id_to_metadata.contains_key(&1));
}

#[test]
fn uppercase_delete_by_id_with_placement_frees_the_whole_image() {
    let _kitty_images = FeatureFlag::KittyImages.override_enabled(true);

    let mut terminal = kitty_terminal();
    place_image(&mut terminal, "i=1,p=1");
    place_image(&mut terminal, "i=1,p=2");

    delete(&mut terminal, "d=I,i=1,p=1");

    assert!(!has_placement(&terminal, 1, 1));
    assert!(!has_placement(&terminal, 1, 2));
}
