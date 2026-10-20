use super::*;
use dicom_core::{DataElement, PrimitiveValue, Tag, VR, value::DataSetSequence};
use dicom_object::FileMetaTableBuilder;
fn put(o: &mut InMemDicomObject, t: Tag, vr: VR, v: &str) {
    let value = if vr == VR::US {
        PrimitiveValue::U16(vec![v.parse().expect("US")].into())
    } else {
        PrimitiveValue::from(v)
    };
    o.put(DataElement::new(t, vr, value));
}
fn fixture(bits: usize, offsets: &str, absolute: bool) -> InMemDicomObject {
    let mut o = InMemDicomObject::new_empty();
    for (tag, vr, v) in [
        (Tag(8, 0x16), VR::UI, RTDOSE_SOP_CLASS_UID),
        (Tag(8, 0x18), VR::UI, "1.2.3.10"),
        (Tag(0x20, 0x52), VR::UI, "1.2.3.4"),
        (Tag(0x3004, 2), VR::CS, "GY"),
        (Tag(0x3004, 4), VR::CS, "PHYSICAL"),
        (Tag(0x3004, 0xa), VR::CS, "PLAN"),
        (Tag(0x3004, 0xe), VR::DS, "0.001"),
        (Tag(0x28, 2), VR::US, "1"),
        (Tag(0x28, 4), VR::CS, "MONOCHROME2"),
        (Tag(0x28, 0x10), VR::US, "2"),
        (Tag(0x28, 0x11), VR::US, "2"),
        (Tag(0x28, 8), VR::IS, "3"),
        (Tag(0x28, 0x103), VR::US, "0"),
        (Tag(0x20, 0x32), VR::DS, "10\\20\\30"),
        (
            Tag(0x20, 0x37),
            VR::DS,
            if absolute {
                "1\\0\\0\\0\\1\\0"
            } else {
                "0\\1\\0\\0\\0\\1"
            },
        ),
        (Tag(0x28, 0x30), VR::DS, "3\\2"),
        (Tag(0x3004, 0xc), VR::DS, offsets),
    ] {
        put(&mut o, tag, vr, v);
    }
    for (tag, v) in [
        (Tag(0x28, 0x100), bits),
        (Tag(0x28, 0x101), bits),
        (Tag(0x28, 0x102), bits - 1),
    ] {
        put(&mut o, tag, VR::US, &v.to_string());
    }
    o.put(DataElement::new(
        Tag(0x28, 9),
        VR::AT,
        PrimitiveValue::from(Tag(0x3004, 0xc)),
    ));
    let mut plan = InMemDicomObject::new_empty();
    put(
        &mut plan,
        Tag(8, 0x1150),
        VR::UI,
        "1.2.840.10008.5.1.4.1.1.481.5",
    );
    put(&mut plan, Tag(8, 0x1155), VR::UI, "1.2.3.20");
    o.put(DataElement::new(
        Tag(0x300c, 2),
        VR::SQ,
        DataSetSequence::from(vec![plan]),
    ));
    let words: Vec<u16> = if bits == 16 {
        (0u16..12).collect()
    } else {
        (100000u32..100012)
            .flat_map(|x| [x as u16, (x >> 16) as u16])
            .collect()
    };
    o.put(DataElement::new(
        Tag(0x7fe0, 0x10),
        VR::OW,
        PrimitiveValue::U16(words.into()),
    ));
    o
}
fn serialized(mut o: InMemDicomObject, ts: &str) -> Vec<u8> {
    if ts == "1.2.840.10008.1.2.2" && integer(&o, "BitsAllocated").expect("bits") == 32 {
        let mut words = o
            .element_by_name("PixelData")
            .expect("pixels")
            .to_multi_int::<u16>()
            .expect("words");
        for pair in words.as_chunks_mut::<2>().0 {
            pair.swap(0, 1);
        }
        o.put(DataElement::new(
            Tag(0x7fe0, 0x10),
            VR::OW,
            PrimitiveValue::U16(words.into()),
        ));
    }

    let f = o
        .with_meta(
            FileMetaTableBuilder::new()
                .media_storage_sop_class_uid(RTDOSE_SOP_CLASS_UID)
                .media_storage_sop_instance_uid("1.2.3.10")
                .transfer_syntax(ts),
        )
        .expect("meta");
    let mut bytes = Vec::new();
    f.write_all(&mut bytes).expect("write");
    bytes
}
#[test]
fn dose_serialized_scaling_geometry_and_32_bit_samples() -> KeplerResult<()> {
    for ts in [
        "1.2.840.10008.1.2.1",
        "1.2.840.10008.1.2",
        "1.2.840.10008.1.2.2",
    ] {
        for bits in [16, 32] {
            let bytes = serialized(fixture(bits, "0\\2\\7", false), ts);
            assert_eq!(rt_sop_class(bytes.as_slice())?, RTDOSE_SOP_CLASS_UID);
            let dose = read_rt_dose(&bytes)?;
            let expected = if bits == 16 { 0.0035 } else { 100.0035 };
            assert!(
                (dose.sample_lps(DVec3::new(11., 21., 31.5)).expect("point") - expected).abs()
                    < 1e-8
            );
            assert_eq!(dose.geometry().spacing, [2., 3.]);
            assert_eq!(dose.metadata().referenced_plan_uids, vec!["1.2.3.20"]);
            assert!(dose.sample_lps(DVec3::new(18., 20., 30.)).is_none());
        }
    }
    Ok(())
}
#[test]
fn dose_absolute_and_descending_frames_remain_physically_located() -> KeplerResult<()> {
    let a = read_rt_dose(&serialized(
        fixture(16, "30\\32\\37", true),
        "1.2.840.10008.1.2.1",
    ))?;
    assert_eq!(a.geometry().frame_offsets, vec![0., 2., 7.]);
    assert_eq!(a.sample_lps(DVec3::new(10., 20., 37.)), Some(0.008));
    let d = read_rt_dose(&serialized(
        fixture(16, "0\\-2\\-7", false),
        "1.2.840.10008.1.2.1",
    ))?;
    assert_eq!(d.geometry().frame_offsets, vec![-7., -2., 0.]);
    assert_eq!(d.sample_lps(DVec3::new(3., 20., 30.)), Some(0.008));
    Ok(())
}
#[test]
fn dose_rejects_malformed_and_unsupported_inputs() {
    for (tag, vr, value) in [
        (Tag(0x3004, 0xc), VR::DS, "0\\2\\2"),
        (Tag(0x3004, 0xc), VR::DS, "0\\2"),
        (Tag(0x3004, 0xc), VR::DS, "1\\2\\3"),
        (Tag(0x3004, 0xe), VR::DS, "NaN"),
        (Tag(0x3004, 0xe), VR::DS, "0"),
        (Tag(0x3004, 4), VR::CS, "ERROR"),
        (Tag(0x3004, 2), VR::CS, "CODED"),
        (Tag(0x28, 0x103), VR::US, "1"),
        (Tag(0x28, 0x11), VR::US, "65535"),
        (Tag(0x28, 8), VR::IS, "1"),
    ] {
        let mut o = fixture(32, "0\\2\\7", false);
        put(&mut o, tag, vr, value);
        assert!(
            read_rt_dose(&serialized(o, "1.2.840.10008.1.2.1")).is_err(),
            "{tag:?}"
        );
    }
    let mut o = fixture(16, "0\\2\\7", false);
    put(&mut o, Tag(0x3004, 2), VR::CS, "RELATIVE");
    assert_eq!(
        read_rt_dose(&serialized(o, "1.2.840.10008.1.2.1"))
            .expect("relative")
            .metadata()
            .units,
        DoseUnits::Relative
    );
}
