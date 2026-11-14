#version 330
// Composed models (buildings and units), instanced. The vertex colour carries the material's
// main colour and, in alpha, its program id plus 128 on faces that wear the owner's colour; the
// tangent carries the second colour and the baked contact shade; the first texture coordinate
// the pattern size. The instance transform's spare cells (m3, m7, m11) carry the owner colour.
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec4 vertexColor;
in vec4 vertexTangent;
in vec2 vertexTexCoord;
in mat4 instanceTransform;
uniform mat4 mvp;
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
    transform[0][3] = 0.0;
    transform[1][3] = 0.0;
    transform[2][3] = 0.0;
    int tag = int(vertexColor.a * 255.0 + 0.5);
    fragTeamFace = tag >= 128 ? 1 : 0;
    fragProgram = tag - fragTeamFace * 128;
    vec4 world = transform * vec4(vertexPosition, 1.0);
    fragWorld = world.xyz;
    fragLocal = vertexPosition;
    fragLocalNormal = vertexNormal;
    // The instance axes are orthogonal and equally scaled, so the normal turns with them.
    fragNormal = mat3(transform) * vertexNormal;
    fragColour = vertexColor.rgb;
    fragAlt = vertexTangent.rgb;
    fragShade = vertexTangent.a;
    fragScale = max(vertexTexCoord.x, 0.01);
    gl_Position = mvp * world;
}
