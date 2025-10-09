const { parentPort } = require("worker_threads");
const ffi = require("ffi-napi");
const ref = require("ref-napi");
const Struct = require("ref-struct-napi");
const ArrayType = require("ref-array-napi");
const path = require("path");
const fs = require("fs");

// ======================
// 定义结构体
// ======================
const CharArray1024 = ArrayType(ref.types.char, 1024);
const DoubleArray3 = ArrayType("double", 3);
const IntArray3 = ArrayType("int", 3);
const FloatArray9 = ArrayType("float", 9);

const Geometry = Struct({
  sid: "double",
  sdd: "double",
  gantry_angle: "double",
  proj_offset_x: "double",
  proj_offset_y: "double",
  out_of_plane_angle: "double",
  in_plane_angle: "double",
  source_offset_x: "double",
  source_offset_y: "double",
  n_projections: "int",
  first_angle: "double",
  arc: "double",
});

const ReconstructionParameter = Struct({
  hnd_path: CharArray1024,
  output_path: CharArray1024,
  output_file: CharArray1024,
  spacing: DoubleArray3,
  dimension: IntArray3,
  origin: DoubleArray3,
  use_gpu: ref.types.bool,
  regexp: CharArray1024,
  direction: FloatArray9,
});

// ======================
// 写字符串工具
// ======================
function setString(charArray, str) {
  const buf = Buffer.from(str + "\0", "utf8");
  for (let i = 0; i < buf.length && i < charArray.length; i++) {
    charArray[i] = buf[i];
  }
}

// ========== DLL ==========
// const dllPath = "C:/Users/admin/Documents/dicom_fromimage/src/rtkfdkdll_1.dll";
const dllPath = "C:/Users/admin/Downloads/rtkfdkdll.dll";
const myDll = ffi.Library(dllPath, {
    rtkfdk: ["int", [ref.refType(ReconstructionParameter), ref.refType(Geometry)]],
});

// 构造 Geometry
let geom = new Geometry();
geom.sid = 1000.0;
geom.sdd = 1500.0;
geom.gantry_angle = 0.0;
geom.n_projections = 360;
geom.first_angle = 0.0;
geom.arc = 360.0;

// 构造 ReconstructionParameter
let param = new ReconstructionParameter();

const absIn = "C:/Users/admin/Documents/dicom_fromimage/src/38-CBCT";
const absOutDir = "C:/Users/admin/Documents/dicom_fromimage/src/38-CBCT";
const absOutFile = "C:/share/input/CT_new.mha";
const mhdIn = "projections_corrected.mhd";

setString(param.hnd_path, absIn);
setString(param.output_path, absOutDir);
setString(param.output_file, absOutFile);
setString(param.regexp, mhdIn);

param.spacing[0] = 0.5;
param.spacing[1] = 0.85;
param.spacing[2] = 0.5;

param.dimension[0] = 512;
param.dimension[1] = 300;
param.dimension[2] = 512;

param.origin[0] = -127.0;
param.origin[1] = -127.0;
param.origin[2] = -127.0;

param.use_gpu = true;

const identity = [0,0,1,1,0,0,0,1,0];
for (let i = 0; i < 9; i++) {
  param.direction[i] = identity[i];
}

// ========== 调 DLL ==========
let ret = myDll.rtkfdk(param.ref(), geom.ref());
parentPort.postMessage({ success: true, code: ret, output: absOutFile });