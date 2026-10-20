use super::*;
use dicom_core::{DataElement, PrimitiveValue, Tag, VR, value::DataSetSequence};
use dicom_object::FileMetaTableBuilder;

fn put(obj: &mut InMemDicomObject, tag: Tag, vr: VR, value: &str) {
    obj.put(DataElement::new(tag, vr, PrimitiveValue::from(value)));
}
fn sequence(obj: &mut InMemDicomObject, tag: Tag, children: Vec<InMemDicomObject>) {
    obj.put(DataElement::new(
        tag,
        VR::SQ,
        DataSetSequence::from(children),
    ));
}
fn fixture(kind: &str, points: &str, count: &str) -> InMemDicomObject {
    let mut object = InMemDicomObject::new_empty();
    put(
        &mut object,
        Tag(0x0008, 0x0016),
        VR::UI,
        RTSTRUCT_SOP_CLASS_UID,
    );
    put(&mut object, Tag(0x0008, 0x0018), VR::UI, "1.2.3.100");
    put(&mut object, Tag(0x3006, 0x0002), VR::SH, "SYNTHETIC");
    let mut roi = InMemDicomObject::new_empty();
    put(&mut roi, Tag(0x3006, 0x0022), VR::IS, "7");
    put(&mut roi, Tag(0x3006, 0x0026), VR::LO, "Synthetic region");
    put(&mut roi, Tag(0x3006, 0x0024), VR::UI, "1.2.3.4");
    sequence(&mut object, Tag(0x3006, 0x0020), vec![roi]);
    let mut image = InMemDicomObject::new_empty();
    put(
        &mut image,
        Tag(0x0008, 0x1150),
        VR::UI,
        "1.2.840.10008.5.1.4.1.1.2",
    );
    put(&mut image, Tag(0x0008, 0x1155), VR::UI, "1.2.3.10");
    let mut contour = InMemDicomObject::new_empty();
    put(&mut contour, Tag(0x3006, 0x0042), VR::CS, kind);
    put(&mut contour, Tag(0x3006, 0x0046), VR::IS, count);
    put(&mut contour, Tag(0x3006, 0x0050), VR::DS, points);
    sequence(&mut contour, Tag(0x3006, 0x0016), vec![image.clone()]);
    let mut contours = InMemDicomObject::new_empty();
    put(&mut contours, Tag(0x3006, 0x0084), VR::IS, "7");
    put(&mut contours, Tag(0x3006, 0x002a), VR::IS, "20\\200\\80");
    sequence(&mut contours, Tag(0x3006, 0x0040), vec![contour]);
    sequence(&mut object, Tag(0x3006, 0x0039), vec![contours]);
    let mut series = InMemDicomObject::new_empty();
    put(&mut series, Tag(0x0020, 0x000e), VR::UI, "1.2.3.5");
    sequence(&mut series, Tag(0x3006, 0x0016), vec![image]);
    let mut study = InMemDicomObject::new_empty();
    sequence(&mut study, Tag(0x3006, 0x0014), vec![series]);
    let mut frame = InMemDicomObject::new_empty();
    put(&mut frame, Tag(0x0020, 0x0052), VR::UI, "1.2.3.4");
    sequence(&mut frame, Tag(0x3006, 0x0012), vec![study]);
    sequence(&mut object, Tag(0x3006, 0x0010), vec![frame]);
    object
}
fn serialized(object: InMemDicomObject) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = object.with_meta(
        FileMetaTableBuilder::new()
            .media_storage_sop_class_uid(RTSTRUCT_SOP_CLASS_UID)
            .media_storage_sop_instance_uid("1.2.3.100")
            .transfer_syntax("1.2.840.10008.1.2.1"),
    )?;
    let mut bytes = vec![];
    file.write_all(&mut bytes)?;
    Ok(bytes)
}
const SQUARE: &str = "1.125\\2.25\\3\\11.125\\2.25\\3\\11.125\\12.25\\3\\1.125\\12.25\\3";

#[test]
fn serialized_rtstruct_preserves_geometry_color_xor_and_references()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = serialized(fixture("CLOSEDPLANAR_XOR", SQUARE, "4"))?;
    let structures = read_rtstruct(&bytes)?;
    assert_eq!(structures.rois[0].number, 7);
    assert_eq!(structures.rois[0].color, [20, 200, 80]);
    assert_eq!(
        structures.rois[0].contours[0].points()[0],
        DVec3::new(1.125, 2.25, 3.)
    );
    assert_eq!(
        structures.rois[0].contours[0].kind(),
        ContourKind::ClosedPlanarXor
    );
    assert_eq!(
        structures.referenced_series[0].series_instance_uid,
        "1.2.3.5"
    );
    let mut identity = DicomVolumeIdentity {
        frame_of_reference_uid: "1.2.3.4".into(),
        series_instance_uid: "1.2.3.5".into(),
        image_uids: vec!["1.2.3.10".into()],
    };
    structures.validate_reference(&identity)?;
    identity.frame_of_reference_uid = "1.2.3.9".into();
    assert!(structures.validate_reference(&identity).is_err());
    identity.frame_of_reference_uid = "1.2.3.4".into();
    identity.series_instance_uid = "1.2.3.99".into();
    assert!(structures.validate_reference(&identity).is_err());
    identity.series_instance_uid = "1.2.3.5".into();
    identity.image_uids.clear();
    assert!(structures.validate_reference(&identity).is_err());
    Ok(())
}

#[test]
fn serialized_malformed_contours_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    for (kind, points, count) in [
        ("CLOSED_PLANAR", SQUARE, "3"),
        (
            "CLOSED_PLANAR",
            "0\\0\\0\\10\\0\\0\\10\\10\\1\\0\\10\\0",
            "4",
        ),
        ("CLOSED_PLANAR", "0\\0\\0\\0\\0\\0\\0\\0\\0", "3"),
        ("CLOSED_PLANAR", "NaN\\0\\0\\10\\0\\0\\10\\10\\0", "3"),
        ("OTHER", SQUARE, "4"),
        ("CLOSED_PLANAR", SQUARE, "500001"),
    ] {
        let bytes = serialized(fixture(kind, points, count))?;
        assert!(read_rtstruct(&bytes).is_err(), "{kind} {count} {points}");
    }
    Ok(())
}

#[test]
fn duplicate_and_undefined_rois_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut object = fixture("CLOSED_PLANAR", SQUARE, "4");
    let roi = items(&object, "StructureSetROISequence")?[0].clone();
    sequence(
        &mut object,
        Tag(0x3006, 0x0020),
        vec![roi.clone(), roi.clone()],
    );
    assert!(read_rtstruct(&serialized(object)?).is_err());
    let mut object = fixture("CLOSED_PLANAR", SQUARE, "4");
    let mut roi = roi;
    put(&mut roi, Tag(0x3006, 0x0022), VR::IS, "9");
    sequence(&mut object, Tag(0x3006, 0x0020), vec![roi]);
    assert!(read_rtstruct(&serialized(object)?).is_err());
    Ok(())
}

#[test]
fn xor_mixtures_and_wrong_sop_class_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut object = fixture("CLOSEDPLANAR_XOR", SQUARE, "4");
    let mut roi = items(&object, "ROIContourSequence")?[0].clone();
    let xor = items(&roi, "ContourSequence")?[0].clone();
    let mut plain = xor.clone();
    put(&mut plain, Tag(0x3006, 0x0042), VR::CS, "CLOSED_PLANAR");
    sequence(&mut roi, Tag(0x3006, 0x0040), vec![xor, plain]);
    sequence(&mut object, Tag(0x3006, 0x0039), vec![roi]);
    assert!(read_rtstruct(&serialized(object)?).is_err());
    let mut object = fixture("CLOSED_PLANAR", SQUARE, "4");
    put(
        &mut object,
        Tag(0x0008, 0x0016),
        VR::UI,
        "1.2.840.10008.5.1.4.1.1.2",
    );
    assert!(read_rtstruct(&serialized(object)?).is_err());
    Ok(())
}

#[test]
fn unknown_vr_contour_payload_is_decoded_as_decimal_text() -> Result<(), Box<dyn std::error::Error>>
{
    let mut object = fixture("CLOSED_PLANAR", SQUARE, "4");
    let mut roi = items(&object, "ROIContourSequence")?[0].clone();
    let mut contour = items(&roi, "ContourSequence")?[0].clone();
    contour.put(DataElement::new(
        Tag(0x3006, 0x0050),
        VR::UN,
        PrimitiveValue::from(SQUARE.as_bytes().to_vec()),
    ));
    sequence(&mut roi, Tag(0x3006, 0x0040), vec![contour]);
    sequence(&mut object, Tag(0x3006, 0x0039), vec![roi]);
    let result = read_rtstruct(&serialized(object)?)?;
    assert_eq!(
        result.rois[0].contours[0].points()[0],
        DVec3::new(1.125, 2.25, 3.)
    );
    Ok(())
}

#[test]
fn serialized_input_limit_is_checked_before_decoding() {
    assert!(read_rtstruct(&vec![0; MAX_RTSTRUCT_BYTES + 1]).is_err());
}
