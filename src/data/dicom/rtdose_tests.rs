use super::*;
use dicom_core::{value::DataSetSequence, DataElement, PrimitiveValue, Tag, VR};
use dicom_object::FileMetaTableBuilder;
fn dvh_item() -> InMemDicomObject {
    let mut o = InMemDicomObject::new_empty();
    for (tag, vr, value) in [
        (Tag(0x3004, 1), VR::CS, "CUMULATIVE"),
        (Tag(0x3004, 2), VR::CS, "GY"),
        (Tag(0x3004, 4), VR::CS, "PHYSICAL"),
        (Tag(0x3004, 0x52), VR::DS, "0.5"),
        (Tag(0x3004, 0x54), VR::CS, "CM3"),
        (Tag(0x3004, 0x56), VR::IS, "4"),
        (Tag(0x3004, 0x58), VR::DS, "2\\100\\3\\100\\5\\50\\10\\0"),
        (Tag(0x3004, 0x70), VR::DS, "0.8"),
        (Tag(0x3004, 0x72), VR::DS, "10"),
        (Tag(0x3004, 0x74), VR::DS, "3.7"),
    ] {
        put(&mut o, tag, vr, value);
    }
    let mut roi = InMemDicomObject::new_empty();
    put(&mut roi, Tag(0x3006, 0x84), VR::IS, "1");
    put(&mut roi, Tag(0x3004, 0x62), VR::CS, "INCLUDED");
    o.put(DataElement::new(
        Tag(0x3004, 0x60),
        VR::SQ,
        DataSetSequence::from(vec![roi]),
    ));
    o
}
fn with_dvhs(mut o: InMemDicomObject, curves: Vec<InMemDicomObject>) -> InMemDicomObject {
    let mut structures = InMemDicomObject::new_empty();
    put(
        &mut structures,
        Tag(8, 0x1150),
        VR::UI,
        crate::data::dicom::RTSTRUCT_SOP_CLASS_UID,
    );
    put(&mut structures, Tag(8, 0x1155), VR::UI, "1.2.3.300");
    o.put(DataElement::new(
        Tag(0x300c, 0x60),
        VR::SQ,
        DataSetSequence::from(vec![structures]),
    ));
    o.put(DataElement::new(
        Tag(0x3004, 0x50),
        VR::SQ,
        DataSetSequence::from(curves),
    ));
    o
}
#[test]
fn optional_dvh_accumulates_scaled_widths_and_keeps_recorded_statistics() -> KeplerResult<()> {
    let bytes = serialized(
        with_dvhs(fixture(16, "0\\2\\7", false), vec![dvh_item()]),
        "1.2.840.10008.1.2.1",
    );
    let grid = read_rt_dose(&bytes)?;
    let dvhs = grid.dvhs().ok_or_else(|| invalid("DVH missing"))?;
    assert!(dvhs.issues.is_empty());
    let curve = &dvhs.curves[0];
    assert_eq!(
        curve.points(),
        [[1., 100.], [2.5, 100.], [5., 50.], [10., 0.]]
    );
    assert_eq!(curve.statistics().mean, Some(3.7));
    assert_eq!(curve.dose_at_percent(95.), Some(2.75));
    assert_eq!(curve.volume_at(6.), Some(40.));
    assert_eq!(grid.samples().len(), 12);
    Ok(())
}
#[test]
fn optional_dvh_errors_preserve_dose_pixels_and_other_curves() -> KeplerResult<()> {
    for mode in 0..6 {
        let mut bad = dvh_item();
        match mode {
            0 => put(&mut bad, Tag(0x3004, 1), VR::CS, "DIFFERENTIAL"),
            1 => put(&mut bad, Tag(0x3004, 0x56), VR::IS, "65537"),
            2 => put(&mut bad, Tag(0x3004, 0x52), VR::DS, "0"),
            3 => put(
                &mut bad,
                Tag(0x3004, 0x58),
                VR::DS,
                "2\\100\\3\\101\\5\\50\\10\\0",
            ),
            4 => put(
                &mut bad,
                Tag(0x3004, 0x58),
                VR::DS,
                "2\\100\\0\\100\\5\\50\\10\\0",
            ),
            _ => put(
                &mut bad,
                Tag(0x3004, 0x58),
                VR::DS,
                "2\\100\\3\\NaN\\5\\50\\10\\0",
            ),
        }
        let grid = read_rt_dose(&serialized(
            with_dvhs(fixture(16, "0\\2\\7", false), vec![bad, dvh_item()]),
            "1.2.840.10008.1.2.1",
        ))?;
        assert_eq!(grid.samples().len(), 12);
        let dvhs = grid.dvhs().ok_or_else(|| invalid("DVH missing"))?;
        assert_eq!(dvhs.curves.len(), 1, "mode {mode}");
        assert_eq!(dvhs.issues.len(), 1, "mode {mode}");
        assert_eq!(dvhs.issues[0].item, Some(1));
    }
    Ok(())
}

#[test]
fn optional_dvh_preserves_composite_roi_contributions_and_percent_units() -> KeplerResult<()> {
    use crate::data::dvh::{DvhContribution, DvhVolumeUnits};
    let mut item = dvh_item();
    put(&mut item, Tag(0x3004, 0x54), VR::CS, "PERCENT");
    let mut refs = item
        .element_by_name("DVHReferencedROISequence")
        .map_err(|_| invalid("refs"))?
        .items()
        .ok_or_else(|| invalid("refs"))?
        .to_vec();
    let mut excluded = refs[0].clone();
    put(&mut excluded, Tag(0x3006, 0x84), VR::IS, "2");
    put(&mut excluded, Tag(0x3004, 0x62), VR::CS, "EXCLUDED");
    refs.push(excluded);
    item.put(DataElement::new(
        Tag(0x3004, 0x60),
        VR::SQ,
        DataSetSequence::from(refs),
    ));
    let grid = read_rt_dose(&serialized(
        with_dvhs(fixture(16, "0\\2\\7", false), vec![item]),
        "1.2.840.10008.1.2.1",
    ))?;
    let source = grid.dvhs().ok_or_else(|| invalid("dvhs"))?;
    assert!(source.issues.is_empty());
    assert_eq!(source.curves[0].volume_units(), DvhVolumeUnits::Percent);
    assert_eq!(source.curves[0].volume_cm3(), None);
    assert_eq!(
        source.curves[0].references()[1].contribution,
        DvhContribution::Excluded
    );
    Ok(())
}

#[test]
fn excessive_dvh_collection_does_not_reject_the_dose_grid() -> KeplerResult<()> {
    let grid = read_rt_dose(&serialized(
        with_dvhs(fixture(16, "0\\2\\7", false), vec![dvh_item(); 257]),
        "1.2.840.10008.1.2.1",
    ))?;
    let source = grid.dvhs().ok_or_else(|| invalid("dvhs"))?;
    assert!(source.curves.is_empty());
    assert_eq!(source.issues.len(), 1);
    assert_eq!(grid.samples().len(), 12);
    Ok(())
}
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
