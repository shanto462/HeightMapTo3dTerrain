use std::io::Write;

use serde_json::{Value, json};

use super::{put_f32s, write_parallel};
use crate::{Error, Mesh, Result};

const ARRAY_BUFFER: u32 = 34_962;
const ELEMENT_ARRAY_BUFFER: u32 = 34_963;
const FLOAT: u32 = 5_126;
const UNSIGNED_INT: u32 = 5_125;

pub(super) fn write(mesh: &Mesh, out: &mut impl Write) -> Result<()> {
    let n = mesh.vertex_count();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut attributes = serde_json::Map::new();
    let mut offset = 0usize;
    let mut add = |name: Option<&str>, bytes: usize, target: u32, accessor: Value| {
        views.push(
            json!({"buffer": 0, "byteOffset": offset, "byteLength": bytes, "target": target}),
        );
        let mut accessor = accessor;
        accessor["bufferView"] = json!(views.len() - 1);
        accessors.push(accessor);
        if let Some(name) = name {
            attributes.insert(name.into(), json!(accessors.len() - 1));
        }
        offset += bytes;
    };

    let (lo, hi) = mesh.bounds();
    add(
        Some("POSITION"),
        12 * n,
        ARRAY_BUFFER,
        json!({"componentType": FLOAT, "count": n, "type": "VEC3", "min": lo, "max": hi}),
    );
    if mesh.normals.is_some() {
        add(
            Some("NORMAL"),
            12 * n,
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": n, "type": "VEC3"}),
        );
    }
    if mesh.uvs.is_some() {
        add(
            Some("TEXCOORD_0"),
            8 * n,
            ARRAY_BUFFER,
            json!({"componentType": FLOAT, "count": n, "type": "VEC2"}),
        );
    }
    let index_count = 3 * mesh.triangle_count();
    add(
        None,
        4 * index_count,
        ELEMENT_ARRAY_BUFFER,
        json!({"componentType": UNSIGNED_INT, "count": index_count, "type": "SCALAR"}),
    );
    let bin_length = offset;
    let indices_accessor = accessors.len() - 1;

    let document = json!({
        "asset": {"version": "2.0", "generator": concat!("hmterrain ", env!("CARGO_PKG_VERSION"))},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "terrain"}],
        "meshes": [{
            "name": "terrain",
            "primitives": [{"attributes": attributes, "indices": indices_accessor, "material": 0, "mode": 4}],
        }],
        "materials": [{
            "name": "terrain",
            "pbrMetallicRoughness": {"baseColorFactor": [0.8, 0.8, 0.8, 1.0], "metallicFactor": 0.0, "roughnessFactor": 1.0},
        }],
        "buffers": [{"byteLength": bin_length}],
        "bufferViews": views,
        "accessors": accessors,
    });
    let mut json = serde_json::to_vec(&document).expect("glTF JSON serializes");
    json.resize(json.len().next_multiple_of(4), b' ');

    // Every block is a multiple of 4 bytes, so the BIN chunk needs no padding.
    let total = 12 + 8 + json.len() + 8 + bin_length;
    let total = u32::try_from(total)
        .map_err(|_| Error::TooLarge("a .glb file is limited to 4 GiB".into()))?;
    out.write_all(b"glTF")?;
    out.write_all(&2u32.to_le_bytes())?;
    out.write_all(&total.to_le_bytes())?;
    out.write_all(&(json.len() as u32).to_le_bytes())?;
    out.write_all(b"JSON")?;
    out.write_all(&json)?;
    out.write_all(&(bin_length as u32).to_le_bytes())?;
    out.write_all(b"BIN\0")?;

    write_parallel(out, &mesh.positions, 12, |buf, _, v| put_f32s(buf, v))?;
    if let Some(normals) = &mesh.normals {
        write_parallel(out, normals, 12, |buf, _, v| put_f32s(buf, v))?;
    }
    if let Some(uvs) = &mesh.uvs {
        write_parallel(out, uvs, 8, |buf, _, v| put_f32s(buf, v))?;
    }
    write_parallel(out, &mesh.indices, 12, |buf, _, tri| {
        for v in tri {
            buf.extend_from_slice(&v.to_le_bytes());
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MeshOptions;
    use crate::export::test_support::sample_mesh;

    fn round_trip(options: &MeshOptions) {
        let mesh = sample_mesh(options);
        let mut out = Vec::new();
        write(&mesh, &mut out).unwrap();
        assert_eq!(out.len() % 4, 0);
        let (doc, buffers, _) = gltf::import_slice(&out).expect("valid glb");
        let primitive = doc.meshes().next().unwrap().primitives().next().unwrap();
        let reader = primitive.reader(|b| Some(&buffers[b.index()]));
        let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
        assert_eq!(positions, mesh.positions);
        let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
        assert_eq!(indices, mesh.indices.concat());
        assert_eq!(
            reader.read_normals().map(Iterator::collect::<Vec<_>>),
            mesh.normals.clone()
        );
        assert_eq!(
            reader
                .read_tex_coords(0)
                .map(|t| t.into_f32().collect::<Vec<_>>()),
            mesh.uvs.clone()
        );
        let (lo, hi) = mesh.bounds();
        let bounds = primitive.bounding_box();
        assert_eq!((bounds.min, bounds.max), (lo, hi));
    }

    #[test]
    fn round_trips_through_a_gltf_reader() {
        round_trip(&MeshOptions::default());
        round_trip(&MeshOptions {
            normals: false,
            uvs: false,
            ..MeshOptions::default()
        });
        round_trip(&MeshOptions {
            base: Some(1.0),
            ..MeshOptions::default()
        });
    }
}
