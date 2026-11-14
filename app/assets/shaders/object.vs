#version 330
// Baked feature objects and crop rows, instanced. The vertex alpha is the program id of the
// part; foliage and crops sway with the wind, more towards their tops. The instance
// transform's spare cells (m3, m7, m11) carry its tint: fog of war times a variation.
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec4 vertexColor;
in mat4 instanceTransform;
uniform mat4 mvp;
uniform float time;
out vec3 fragNormal;
out vec3 fragWorld;
out vec3 fragColour;
flat out vec3 fragTint;
flat out int fragProgram;
void main() {
    mat4 transform = instanceTransform;
    fragTint = vec3(transform[0][3], transform[1][3], transform[2][3]);
    transform[0][3] = 0.0;
    transform[1][3] = 0.0;
    transform[2][3] = 0.0;
    int program = int(vertexColor.a * 255.0 + 0.5);
    vec3 local = vertexPosition;
    if (program == 7 || program == 9) {
        float phase = dot(transform[3].xyz, vec3(0.37, 0.71, 0.53));
        float bend = max(local.y, 0.0);
        bend *= bend;
        float amount = program == 9 ? 0.16 : 0.03;
        float gust = 0.6 + 0.4 * sin(time * 0.6 + phase * 0.3);
        local.x += sin(time * 1.6 + phase) * amount * gust * bend;
        local.z += cos(time * 1.2 + phase * 1.3) * amount * 0.5 * gust * bend;
    }
    vec4 world = transform * vec4(local, 1.0);
    fragWorld = world.xyz;
    // The instance axes are orthogonal (a turn and a scale per axis), so the inverse transpose
    // is each axis divided by its squared length.
    vec3 ax = transform[0].xyz;
    vec3 ay = transform[1].xyz;
    vec3 az = transform[2].xyz;
    fragNormal = ax * (vertexNormal.x / dot(ax, ax)) + ay * (vertexNormal.y / dot(ay, ay))
               + az * (vertexNormal.z / dot(az, az));
    fragColour = vertexColor.rgb;
    fragProgram = program;
    gl_Position = mvp * world;
}
