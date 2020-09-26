#version 450
#extension GL_ARB_separate_shader_objects : enable

precision highp float;

layout(location = 0) noperspective in vec2 cf;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 1) uniform Locals {
    dvec2 alpha;
    dvec2 delta;
};

layout(set = 0, binding = 2) uniform Iters {
    int iter;
};

void main() {
    dvec2 cd = cf;
    dvec2 c = dvec2(alpha.x * cd.x - alpha.y * cd.y, alpha.x * cd.y + alpha.y * cd.x) + delta;

    dvec2 z = c;
    int i;
    for(i=0; i<iter; i++) {
        double x = (z.x * z.x - z.y * z.y) + c.x;
        double y = (z.y * z.x + z.x * z.y) + c.y;

        if((x * x + y * y) > 4.0) break;
        z.x = x;
        z.y = y;
    }

    float r = (i == iter) ? 0.0 : float(i) / iter;
    float g = r * r;
    float b = g * g;
    outColor = vec4(r, g, b, 1.0);
}