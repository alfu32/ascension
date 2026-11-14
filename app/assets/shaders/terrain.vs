#version 330
// Land chunks. The vertex colour is the map colour already darkened by fog of war and tinted
// by borders; texcoord.x carries the unshaded map colour packed as r*65536 + g*256 + b, so the
// shade can be applied to the material colours too. Seven material weights ride in
// tangent (slots 0-3), texcoord2 (4-5) and texcoord.y (6).
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec4 vertexColor;
in vec2 vertexTexCoord;
in vec2 vertexTexCoord2;
in vec4 vertexTangent;
uniform mat4 mvp;
uniform mat4 matModel;
out vec3 fragNormal;
out vec4 fragColor;
out vec3 fragWorld;
out vec3 fragShade;
out vec4 fragWeightsA;
out vec3 fragWeightsB;
void main() {
    float code = vertexTexCoord.x;
    vec3 base = vec3(floor(code / 65536.0), mod(floor(code / 256.0), 256.0), mod(code, 256.0)) / 255.0;
    fragShade = clamp(vertexColor.rgb / max(base, vec3(1.0 / 255.0)), 0.0, 2.0);
    fragWeightsA = vertexTangent;
    fragWeightsB = vec3(vertexTexCoord2, vertexTexCoord.y);
    fragNormal = vertexNormal;
    fragColor = vertexColor;
    fragWorld = (matModel * vec4(vertexPosition, 1.0)).xyz;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
