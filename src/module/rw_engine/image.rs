use bracket_color::{hsv::HSV, rgb::RGB};
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use std::path::Path;

use crate::packet::{super_packet::SuperPacket, Packet};

pub async fn get_image_packet(image_path: &Path) -> anyhow::Result<Vec<Packet>> {
    let img = ImageReader::open(image_path)?.decode()?;
    let mut packets = Vec::new();
    for (x, y, rgba) in img.pixels() {
        //let packet = SuperPacket::set_terrain(x as f32 * 20., y as f32 * 20.).await;
        let rgb = rgba.to_rgb();
        let rgb = RGB::from_u8(rgb.0[0], rgb.0[1], rgb.0[2]);
        let hsv = rgb.to_hsv();
        let terrain = get_hsv_terrain_name(hsv);

        packets.push(SuperPacket::set_terrain(x as f32 * 20., y as f32 * 20., &terrain).await);
    }
    Ok(packets)
}

pub fn get_hsv_terrain_name(hsv: HSV) -> String {
    let h = (hsv.h * 360.) as u32;
    let s = (hsv.s * 100.) as u32;
    let v = (hsv.v * 100.) as u32;

    let h = h.min(360);
    let s = s.min(95);
    let v = v.min(95);

    let h = h - h % 5;
    let s = s - s % 5;
    let v = v - v % 5;

    format!("hsv{:0>3}{:0>2}{:0>2}", h, s, v)
}
