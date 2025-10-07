#version 300 es
    precision highp float;
    precision highp sampler3D;
    // precision mediump sampler2D;

    uniform sampler3D sampler0;
    // uniform sampler3D sampler1;
    uniform sampler3D sampler2;
    uniform sampler3D sampler3  ;
    uniform sampler3D sampler4;
    uniform float window;
    uniform float level;
    uniform float window1;
    uniform float level1;
    uniform float st;
    uniform float ss;
    uniform float sc;
    uniform lowp int type;
    uniform float ptx;
    uniform float pty;
    uniform float psx;
    uniform float psy;
    uniform float pcx;
    uniform float pcy;
    uniform float slt;
    uniform float sls;
    uniform float slc;
    // uniform float slope0;
    // uniform float intercept0;
    // uniform float slope1;
    // uniform float intercept1;
    uniform float k;            // blend coefficient
    uniform int s;            // 0: primary 1: primary/secondary
    uniform int lut;          // 0: off  1: on
    uniform vec3 spacing0;
    uniform vec3 spacing1;
    uniform vec3 dim0;
    uniform vec3 dim1;
    uniform vec3 shift;
    uniform vec3 size0;
    uniform vec3 size1;

    // Ablation
    uniform vec3 uah;     // (u0, a, h) for Ablation
    uniform float L;      // Length of the needle
    uniform vec3 npos;
    uniform float theta;

    in vec3 pos;
    out vec4 color;

    float rgba2u16(vec4 rgba) {
        return 15.0 * (rgba.r * 4096.0 + rgba.g * 256.0 + rgba.b * 16.0 + rgba.a);
    }

    float rgba2f32(vec4 rgba) {
        return rgba2u16(rgba) / 65535.0;
    }

    vec3 project_t3(vec3 p, vec3 pan, vec3 scale) {
        const mat4x3 M = mat4x3(0.5, 0.0, 0.0, 0.0, -0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5);
        vec3 q = (p - pan) / scale;
        return M * vec4(q, 1.0);
    }
    vec3 project_s3(vec3 p, vec3 pan, vec3 scale) {
        const mat4x3 M = mat4x3(0.5, 0.0, 0.0, 0.0, -0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5);
        vec3 q = (p - pan) / scale;
        return M * vec4(q, 1.0);
    }
    vec3 project_c3(vec3 p, vec3 pan, vec3 scale) {
        const mat4x3 M = mat4x3(0.5, 0.0, 0.0, 0.0, -0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.5);
        vec3 q = (p - pan) / scale;
        return M * vec4(q, 1.0);
    }

    float minv3(vec3 v) {
        return min(min(v[0], v[1]), v[2]);
    }
    float maxv3(vec3 v) {
        return max(max(v[0], v[1]), v[2]);
    }

    bool is_outbound(float x, float y, float z) {
        return (x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 || z < 0.0 || z > 1.0 ); 
    }
    bool is_outbound(vec3 p) {
        return (p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0 || p.z < 0.0 || p.z > 1.0 ); 
    }

    // Ablation
    float calc_dose(float u0, float a, float h, float x, float y) {
       // return (x * x + y * y) * 2000.0;
        float delta = 0.00000001;
        float b2 = h*h - a*a;
        float b = sqrt(b2);
        float ha = h - a;
        float ln = log((b + ha) / (b - ha));
        float x2 = x * x;
        float y2 = y * y;
        float xy = ((x + b)*(x + b) + y2) * ((x - b) * (x - b) + y2);
        float d = ln * ln * xy;
        // d = max(delta, d);
        float E2 = 4.0 * b2 * u0 * u0 / d;
        float E = sqrt(E2);
        return E;
    }

    // float asinh(float x) {
    //     return log(x + sqrt(x * x + 1));
    // }

    float d_2(float x) {
        return x * x;
    }

    float calc_E(float U0, float R, float L, float h, float x, float y, float z) {
        float W = U0 / (4.0*asinh(L/(2.0*sqrt(R*R))));
        float hh = h / 2.0;
        float hL = L / 2.0;
        float E = sqrt(d_2(-W*(-1.0/(sqrt(y*y + d_2(-hh + x))*sqrt(d_2(hL - z)/(y*y + d_2(-hh + x)) + 1.0)) + \
                  1.0/(sqrt(y*y + d_2(-hh + x))*sqrt(d_2(-hL - z)/(y*y + d_2(-hh + x)) + 1.0))) + \
              W*(-1.0/(sqrt(y*y + d_2(hh + x))*sqrt(d_2(hL - z)/(y*y + d_2(hh + x)) + 1.0)) + \
                 1.0/(sqrt(y*y + d_2(hh + x))*sqrt(d_2(-hL - z)/(y*y + d_2(hh + x)) + 1.0)))) + \
             d_2(-W*(y*(-hL - z)/(pow(y*y + d_2(-hh + x), 1.5)*sqrt(d_2(-hL - z)/(y*y + d_2(-hh + x)) + 1.0)) - \
                  y*(hL - z)/(pow(y*y + d_2(-hh + x), 3.0/2.0)*sqrt(d_2(hL - z)/(y*y + d_2(-hh + x)) + 1.0))) + \
              W*(y*(-hL - z)/(pow(y*y + d_2(hh + x), 3.0/2.0)*sqrt(d_2(-hL - z)/(y*y + d_2(hh + x)) + 1.0)) - \
                 y*(hL - z)/(pow(y*y + d_2(hh + x), 3.0/2.0)*sqrt(d_2(hL - z)/(y*y + d_2(hh + x)) + 1.0)))) + \
                 d_2(W*(-(-hL - z)*(-hh - x)/(pow(y*y + d_2(hh + x), 3.0/2.0)*sqrt(d_2(-hL - z)/(y*y + d_2(hh + x)) + 1.0)) + \
                     (hL - z)*(-hh - x)/(pow(y*y + d_2(hh + x), 1.5)*sqrt(d_2(hL - z)/(y*y + \
            d_2(hh + x)) + 1.0))) - W*(-(-hL - z)*(hh - x)/(pow(y*y + d_2(-hh + x), 1.5)*sqrt(d_2(-hL - z)/(y*y + \
            d_2(-hh + x)) + 1.0)) + (hL - z)*(hh - x)/(pow(y*y + d_2(-hh + x), 3.0/2.0)*sqrt(d_2(hL - z)/(y*y + d_2(-hh + x)) + 1.0)))));
        return E;
    }

    void main() {
        vec3 coord0, coord1;
        float u, v, w;
        float u1, v1, w1;
        // vec3 n0 = dim0 * spacing0;
        // vec3 n1 = dim1 * spacing1;
        vec3 n0 = size0;
        vec3 n1 = size1;
        
        float d = 500.0;
        if (type == 0) {
            float z0 = rgba2f32(texture(sampler3, vec3((slt + 1.0) / 2.0, 0.5, 0.0))) * 2.0 - 1.0;
            vec3 m = vec3(pos.xy, z0);
            vec3 np0 = m / n0 * d;
            vec3 scale = vec3(st, st, 1.0);

            coord0 = project_t3(np0, vec3(ptx, pty, 0.0)/n0*d, scale);
        } 
        else if (type == 1) {
            vec3 m = vec3(sls, pos.xy);
            vec3 np0 = m / n0 * d;
            vec3 scale = vec3(1.0, ss, ss);
            coord0 = project_s3(np0, vec3(0.0, psx, psy)/n0*d, scale);
        }
        else if (type == 2) {
            float z = rgba2f32(texture(sampler3, vec3((pos.y + 1.0) / 2.0, 0.5, 0.0))) * 2.0 - 1.0;
            vec3 m = vec3(pos.x, slc, z);
            vec3 np0 = m / n0 * d;
            vec3 scale = vec3(sc, 1.0, sc);
            coord0 = project_c3(np0, vec3(pcx, 0.0, pcy)/n0*d, scale);
        }
        float lower = level - window / 2.0;
        float upper = level + window / 2.0;

        vec4 pixel0 = texture(sampler0, coord0);
        float value0 = rgba2u16(pixel0);
        float gray0 = clamp((value0 - lower) / window, 0.0, 1.0);
        if (is_outbound(coord0)) {
            gray0 = 0.0;
        }

        color = vec4(0.6, 0.1, 0.2, 1.0);
        float upper1 = level1 + window1 / 2.0;
        float lower1 = level1 - window1 / 2.0;
        mat2x2 R = mat2x2(cos(theta), -sin(theta), sin(theta), cos(theta));
        if (type == 0) {
            // float value1 = calc_dose(uah.x, uah.y, uah.z, pos.x * d / st, pos.y * d / st);
            vec3 p = vec3((pos.x - ptx) * d / st - npos.x, (pos.y - pty) * d / st - npos.y, slt * d - npos.z);
            p = vec3(R * vec2(p.xy), p.z);
            float value1 = calc_E(uah.x, uah.y, L, uah.z*2.0, p.x, p.y, p.z);
            float gray1 = clamp((value1 - lower1) / window1, 0.0, 1.0);
            if (is_outbound(coord1)) {
                gray1 = 0.0;
            }
            vec4 scolor = texture(sampler2, vec3(gray1, 0.5, 0.0));
            color = vec4((gray0 * (1.0 - k) + scolor.xyz * k), 1.0);
        } else if (type == 1) {
            vec3 p = vec3(sls * d - npos.x, (pos.x - psx) * d / ss - npos.y, (pos.y - psy) * d / ss - npos.z);
            p = vec3(R * vec2(p.xy), p.z);
            float value1 = calc_E(uah.x, uah.y, L, uah.z*2.0, p.x, p.y, p.z);
            float gray1 = clamp((value1 - lower1) / window1, 0.0, 1.0);
            if (is_outbound(coord1)) {
                gray1 = 0.0;
            }
            vec4 scolor = texture(sampler2, vec3(gray1, 0.5, 0.0));
            color = vec4((gray0 * (1.0 - k) + scolor.xyz * k), 1.0);
        } else if (type == 2) {
            vec3 p = vec3((pos.x - pcx) * d / sc - npos.x, slc * d - npos.y, (pos.y - pcy) * d / sc - npos.z);
            p = vec3(R * vec2(p.xy), p.z);
            float value1 = calc_E(uah.x, uah.y, L, uah.z*2.0, p.x, p.y, p.z);
            float gray1 = clamp((value1 - lower1) / window1, 0.0, 1.0);
            if (is_outbound(coord1)) {
                gray1 = 0.0;
            }
            vec4 scolor = texture(sampler2, vec3(gray1, 0.5, 0.0));
            color = vec4((gray0 * (1.0 - k) + scolor.xyz * k), 1.0);
        } else {
            color = vec4(vec3(gray0), 1.0);
        }
        // if (type == 0) {
        //     float x0 = (pos.x - ptx) * d / st - npos.x;
        //     float y0 = (pos.y - pty) * d / st - npos.y;
        //     float z0 = slt * d - npos.z;
        //     float e0 = calc_E(uah.x, uah.y, L, uah.z * 2.0, x0 - 1.0, y0 + 1.0, z0);
        //     float e1 = calc_E(uah.x, uah.y, L, uah.z * 2.0, x0 + 1.0, y0 + 1.0, z0);
        //     float e2 = calc_E(uah.x, uah.y, L, uah.z * 2.0, x0 - 1.0, y0 - 1.0, z0);
        //     float e3 = calc_E(uah.x, uah.y, L, uah.z * 2.0, x0 + 1.0, y0 - 1.0, z0);
        //     if (e0 > 80.0 && e1 > 80.0 && e2 > 80.0 && e3 > 80.0) {
        //         color = vec4(vec3(gray0), 1.0);
        //     } else if (e0 < 80.0 && e1 < 80.0 && e2 < 80.0 && e3 < 80.0) {
        //         color = vec4(vec3(gray0), 1.0);
        //     } else {
        //         color = vec4(1.0, 0.0, 0.0, 1.0);
        //     }
        // } else {
        //     color = vec4(vec3(gray0), 1.0);
        // }

    