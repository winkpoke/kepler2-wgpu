/* tslint:disable */
/* eslint-disable */

export class CTVolume {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    export_mha(): Uint8Array;
}

export class DicomObject {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static from_bytes(bytes: Uint8Array): DicomObject;
}

export class DicomRepo {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    generate_ct_volume(image_series_id: string): CTVolume;
    get_all_patients(): string;
    get_images_by_series(series_id: string): string;
    get_patient(patient_id: string): string;
    get_series_by_study(study_id: string): string;
    get_studies_by_patient(patient_id: string): string;
}

export class GLCanvas {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    apply_segmentation(raw: Uint8Array): void;
    clear_layout(): void;
    export_current_obj(): Promise<string>;
    /**
     * Export one standalone OBJ per loaded label.
     */
    export_split_obj(): Promise<Array<any>>;
    get_base_screen(index: number): Promise<Float32Array>;
    get_oblique_normal(index: number): Promise<Float32Array>;
    get_pan(index: number): Promise<Float32Array>;
    get_pixel_value_from_screen(index: number, x: number, y: number): Promise<Float32Array>;
    get_rotation(index: number): Promise<string>;
    get_screen_coord_in_mm(index: number, x: number, y: number, z: number): Promise<Float32Array>;
    get_window_level(index: number): Promise<Float32Array>;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    handle_view_click(index: number, screen_x: number, screen_y: number, screen_z: number): void;
    /**
     * Dispatch a view click and asynchronously return computed world/slice data.
     */
    handle_view_click_get(index: number, x: number, y: number, z: number): Promise<Float32Array>;
    /**
     * Sends an InvalidatePipelines event without any shader changes.
     */
    invalidate_pipelines(): void;
    load_data_from_ct_volume(volume: CTVolume): void;
    /**
     * Stack a full-range pseudo-color overlay built from `overlay_vol`
     * (a separate MHA) onto the DR MPR view in grid cell 0. Call after
     * `load_dr_to_pro`; geometry lines up only when the overlay volume
     * shares the DR volume's voxel grid.
     */
    load_dr_overlay(overlay_vol_0: CTVolume, overlay_vol_90: CTVolume, scale: number): void;
    load_dr_to_pro(index: number, vol: CTVolume, avg_dark: Uint8Array, avg_bright: Uint8Array): void;
    quit(): void;
    /**
     * Sends a ReloadShaders event which will invalidate pipelines; they will be recreated as needed.
     */
    reload_shaders(): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    reset_mesh(index: number): void;
    resize(width: number, height: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_aliasing(index: number, aliasing: boolean): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_ball_enabled(index: number, enabled: boolean): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_ball(index: number, id: number, x: number, y: number, z: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_mesh_mode(index: number, mode: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_mesh_needle_enabled(index: number, enabled: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_mesh_opacity(index: number, alpha: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_mesh_roi(index: number, sx: number, sy: number, sz: number, lx: number, ly: number, lz: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_mesh_rotation_enabled(index: number, enabled: boolean): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_mip_mode(index: number, mode: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_needle_angle(index: number, id: number, angle: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_needle_position(index: number, id: number, x: number, y: number, z: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_needle_radius(index: number, id: number, radius: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_new_needle(index: number, id: number, x: number, y: number, z: number, lx: number, ly: number, lz: number, r: number, g: number, b: number): void;
    set_obj_mesh(raw: Uint8Array, id: number, kind: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_oblique_rotation_radians(index: number, horizontal_radians: number, vertical_radians: number, in_plane_radians: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_pan(index: number, dx: number, dy: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_pan_mm(index: number, dx_mm: number, dy_mm: number): void;
    set_render_mode(mode: number, mesh_index: number | null | undefined, mpr_index: number | null | undefined, mip_index: number | null | undefined, orientation_index: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_rotation_angle_degrees(index: number, roll_deg: number, yaw_deg: number, pitch_deg: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_rotation_degrees(index: number, degrees_x: number, degrees_y: number): void;
    set_rotation_quat(index: number, quat: Float32Array): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_scale(index: number, scale: number): void;
    /**
     * Send a label-visibility mask update to the render loop. `mask` is an
     * 8-element array (index = label id, value > 0.5 means visible). Only the
     * first 8 entries are used; a shorter slice leaves the rest hidden.
     */
    set_segmentation_visibility(mask: Float32Array): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_slab_thickness(index: number, thickness: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_slice_mm(index: number, slice: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_translate_in_screen_coord(index: number, x: number, y: number, z: number): void;
    set_window_by_div_id(div_id: string, volume: CTVolume): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_window_level(index: number, window_level: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    set_window_width(index: number, window_width: number): void;
    /**
     * Sends a `UserEvent::$variant` targeted to a specific window index.
     */
    sync_oblique_to_3d(index: number, oblique_crop: number, alpha: number): void;
    world_coord_to_screen(index: number, x: number, y: number, z: number): Promise<Float32Array>;
}

export class Graphics {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
}

export class RemedyWasm {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Build a command packet to send to Serial Port
     */
    build_command(cmd: string): Uint8Array;
    confirm_response(): void;
    get_retry_timeout_ms(): number;
    /**
     * Get current system state
     */
    get_state(): any;
    /**
     * Check if protocol is in standby mode (for heartbeat)
     */
    is_standby(): boolean;
    constructor();
    /**
     * Process incoming bytes from Serial Port (passed as Uint8Array from JS)
     */
    process_input(input: Uint8Array): any;
    /**
     * Reset the protocol state
     */
    reset(): void;
    retry_on_timeout(): Uint8Array;
    send_command_with_retry(cmd: string): Uint8Array;
}

export class RenderApp {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    get_glcanvas(): GLCanvas;
    run(): Promise<void>;
}

export function build_ct_dicom_wasm(mha_bytes: Uint8Array, data_bytes: Uint8Array | null | undefined, patient: Uint8Array, study: Uint8Array, info: Uint8Array): Promise<any>;

export function drop_render_app(app: RenderApp): void;

export function get_render_app(): Promise<RenderApp>;

export function init(): Promise<void>;

export function load_data_from_repo_wasm(image_series_number: string): void;

/**
 * Function-level comment: Parses common medical imaging files (MHA, MHD, etc.) for WASM
 * Uses the unified MedicalImageParser interface to support multiple formats
 * For MHD files, expects both header (.mhd) and data (.raw/.zraw) files in the array
 */
export function parse_common_files_wasm(files: Array<any>, info: Uint8Array): Promise<CTVolume>;

export function parse_dcm_files_wasm(files: Array<any>): Promise<DicomRepo>;

export function parse_mha_and_generate_ct(header_bytes: Uint8Array, data_bytes: Uint8Array | null | undefined, slope: number, intercept: number): Promise<CTVolume>;

export function read_file_as_bytes(file: File): Promise<Uint8Array>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ctvolume_free: (a: number, b: number) => void;
    readonly __wbg_dicomobject_free: (a: number, b: number) => void;
    readonly __wbg_dicomrepo_free: (a: number, b: number) => void;
    readonly __wbg_glcanvas_free: (a: number, b: number) => void;
    readonly __wbg_graphics_free: (a: number, b: number) => void;
    readonly __wbg_remedywasm_free: (a: number, b: number) => void;
    readonly __wbg_renderapp_free: (a: number, b: number) => void;
    readonly build_ct_dicom_wasm: (a: any, b: number, c: any, d: any, e: any) => any;
    readonly ctvolume_export_mha: (a: number) => any;
    readonly dicomobject_from_bytes: (a: number, b: number) => [number, number, number];
    readonly dicomrepo_generate_ct_volume: (a: number, b: number, c: number) => [number, number, number];
    readonly dicomrepo_get_all_patients: (a: number) => [number, number, number, number];
    readonly dicomrepo_get_images_by_series: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dicomrepo_get_patient: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dicomrepo_get_series_by_study: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dicomrepo_get_studies_by_patient: (a: number, b: number, c: number) => [number, number, number, number];
    readonly drop_render_app: (a: number) => void;
    readonly get_render_app: () => any;
    readonly glcanvas_apply_segmentation: (a: number, b: number, c: number) => void;
    readonly glcanvas_clear_layout: (a: number) => void;
    readonly glcanvas_export_current_obj: (a: number) => any;
    readonly glcanvas_export_split_obj: (a: number) => any;
    readonly glcanvas_get_base_screen: (a: number, b: number) => any;
    readonly glcanvas_get_oblique_normal: (a: number, b: number) => any;
    readonly glcanvas_get_pan: (a: number, b: number) => any;
    readonly glcanvas_get_pixel_value_from_screen: (a: number, b: number, c: number, d: number) => any;
    readonly glcanvas_get_rotation: (a: number, b: number) => any;
    readonly glcanvas_get_screen_coord_in_mm: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly glcanvas_get_window_level: (a: number, b: number) => any;
    readonly glcanvas_handle_view_click: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly glcanvas_handle_view_click_get: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly glcanvas_invalidate_pipelines: (a: number) => void;
    readonly glcanvas_load_data_from_ct_volume: (a: number, b: number) => void;
    readonly glcanvas_load_dr_overlay: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_load_dr_to_pro: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly glcanvas_quit: (a: number) => void;
    readonly glcanvas_reload_shaders: (a: number) => void;
    readonly glcanvas_reset_mesh: (a: number, b: number) => void;
    readonly glcanvas_resize: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_aliasing: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_ball: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly glcanvas_set_ball_enabled: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_mesh_mode: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_mesh_needle_enabled: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_mesh_opacity: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_mesh_roi: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly glcanvas_set_mesh_rotation_enabled: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_mip_mode: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_needle_angle: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_needle_position: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly glcanvas_set_needle_radius: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_new_needle: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => void;
    readonly glcanvas_set_obj_mesh: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly glcanvas_set_oblique_rotation_radians: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly glcanvas_set_pan: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_pan_mm: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_render_mode: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly glcanvas_set_rotation_angle_degrees: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly glcanvas_set_rotation_degrees: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_rotation_quat: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_scale: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_segmentation_visibility: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_slab_thickness: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_slice_mm: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_translate_in_screen_coord: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly glcanvas_set_window_by_div_id: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_set_window_level: (a: number, b: number, c: number) => void;
    readonly glcanvas_set_window_width: (a: number, b: number, c: number) => void;
    readonly glcanvas_sync_oblique_to_3d: (a: number, b: number, c: number, d: number) => void;
    readonly glcanvas_world_coord_to_screen: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly init: () => void;
    readonly load_data_from_repo_wasm: (a: number, b: number) => void;
    readonly parse_common_files_wasm: (a: any, b: any) => any;
    readonly parse_dcm_files_wasm: (a: any) => any;
    readonly parse_mha_and_generate_ct: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
    readonly read_file_as_bytes: (a: any) => any;
    readonly remedywasm_build_command: (a: number, b: number, c: number) => [number, number];
    readonly remedywasm_confirm_response: (a: number) => void;
    readonly remedywasm_get_retry_timeout_ms: (a: number) => number;
    readonly remedywasm_get_state: (a: number) => any;
    readonly remedywasm_is_standby: (a: number) => number;
    readonly remedywasm_new: () => number;
    readonly remedywasm_process_input: (a: number, b: number, c: number) => any;
    readonly remedywasm_reset: (a: number) => void;
    readonly remedywasm_retry_on_timeout: (a: number) => [number, number];
    readonly remedywasm_send_command_with_retry: (a: number, b: number, c: number) => [number, number];
    readonly renderapp_get_glcanvas: (a: number) => number;
    readonly renderapp_run: (a: number) => any;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___js_sys_7f92bae05160e1d5___Array__web_sys_18888586e2f105e6___features__gen_ResizeObserver__ResizeObserver______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___js_sys_7f92bae05160e1d5___Function__js_sys_7f92bae05160e1d5___Function______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_ProgressEvent__ProgressEvent__core_dd2ae236e7a26be___result__Result_____alloc_a265129b4308bccd___string__String___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___js_sys_7f92bae05160e1d5___Array______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___wasm_bindgen_888ac3f26cce4c7___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_Event__Event______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_FocusEvent__FocusEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_KeyboardEvent__KeyboardEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_PageTransitionEvent__PageTransitionEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_PointerEvent__PointerEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___web_sys_18888586e2f105e6___features__gen_WheelEvent__WheelEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke___wgpu_2f7a8738a2336274___backend__webgpu__webgpu_sys__gen_GpuUncapturedErrorEvent__GpuUncapturedErrorEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_888ac3f26cce4c7___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
