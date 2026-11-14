#version 330
// Composed models (buildings and units), instanced. The vertex colour carries the material's
// main colour and, in alpha, its program id plus 128 on faces that wear the owner's colour; the
// tangent carries the second colour and the baked contact shade; the first texture coordinate
// the pattern size and the joint the vertex moves with. The instance transform's spare cells
// (m3, m7, m11) carry the owner colour, its last cell (m15) the activity plus the phase.
// A moving vertex is carried by its joint's transform, blended between the two keyframes of
// the pose texture around this moment of the activity's cycle.
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec4 vertexColor;
in vec4 vertexTangent;
in vec2 vertexTexCoord;
in mat4 instanceTransform;
uniform mat4 mvp;
uniform sampler2D poseTexture;
// Texel of this model's keyframes (x < 0: the model does not move), samples per cycle,
// cycle lengths of idle, move, attack and work (seconds) and the time shown.
uniform ivec2 poseOrigin;
uniform int poseSamples;
uniform vec4 posePeriods;
uniform float poseTime;
out vec3 fragNormal;
out vec3 fragWorld;
out vec3 fragLocal;
out vec3 fragLocalNormal;
out vec3 fragColour;
out vec3 fragAlt;
out float fragShade;
flat out float fragScale;
flat out vec3 fragTeam;
flat out int fragProgram;
flat out int fragTeamFace;
void main() {
    mat4 transform = instanceTransform;
    fragTeam = vec3(transform[0][3], transform[1][3], transform[2][3]);
    float pose = transform[3][3];
    transform[0][3] = 0.0;
    transform[1][3] = 0.0;
    transform[2][3] = 0.0;
    transform[3][3] = 1.0;
    int tag = int(vertexColor.a * 255.0 + 0.5);
    fragTeamFace = tag >= 128 ? 1 : 0;
    fragProgram = tag - fragTeamFace * 128;
    vec3 position = vertexPosition;
    vec3 normal = vertexNormal;
    int joint = int(vertexTexCoord.y + 0.5);
    if (joint > 0 && poseOrigin.x >= 0) {
        int activity = clamp(int(floor(pose)), 0, 3);
        float cycle = fract(poseTime / max(posePeriods[activity], 0.05) + fract(pose));
        float at = cycle * float(poseSamples);
        int s0 = int(floor(at)) % poseSamples;
        int s1 = (s0 + 1) % poseSamples;
        float blend = at - floor(at);
        int x = poseOrigin.x + joint * 3;
        int y = poseOrigin.y + activity * poseSamples;
        vec4 r0 = mix(texelFetch(poseTexture, ivec2(x, y + s0), 0), texelFetch(poseTexture, ivec2(x, y + s1), 0), blend);
        vec4 r1 = mix(texelFetch(poseTexture, ivec2(x + 1, y + s0), 0), texelFetch(poseTexture, ivec2(x + 1, y + s1), 0), blend);
        vec4 r2 = mix(texelFetch(poseTexture, ivec2(x + 2, y + s0), 0), texelFetch(poseTexture, ivec2(x + 2, y + s1), 0), blend);
        vec4 p = vec4(vertexPosition, 1.0);
        position = vec3(dot(r0, p), dot(r1, p), dot(r2, p));
        normal = vec3(dot(r0.xyz, vertexNormal), dot(r1.xyz, vertexNormal), dot(r2.xyz, vertexNormal));
    }
    vec4 world = transform * vec4(position, 1.0);
    fragWorld = world.xyz;
    // Patterns stay on the part as it moves: they follow the model at rest.
    fragLocal = vertexPosition;
    fragLocalNormal = vertexNormal;
    // The instance axes are orthogonal and equally scaled, so the normal turns with them.
    fragNormal = mat3(transform) * normal;
    fragColour = vertexColor.rgb;
    fragAlt = vertexTangent.rgb;
    fragShade = vertexTangent.a;
    fragScale = max(vertexTexCoord.x, 0.01);
    gl_Position = mvp * world;
}
