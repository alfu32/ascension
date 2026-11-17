#version 330
// Stains on the ground (task 80). Soot vertices are copies of the terrain's own: they are drawn
// a hair nearer the eye than the ground (along the view ray, so nothing moves on screen) and
// write no depth. Debris pieces settle into the ground as the stain fades. texcoord: game
// seconds the stain was left at, and its seed; texcoord2: the offset from its centre in radii
// (soot), or a mark above 8 and the height above the ground (debris).
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec4 vertexColor;
in vec2 vertexTexCoord;
in vec2 vertexTexCoord2;
uniform mat4 mvp;
uniform vec3 viewPosition;
uniform float now;
uniform float life;
/*LIGHTING*/
out vec3 fragWorld;
out vec3 fragNormal;
out vec4 fragColour;
out vec2 fragLocal;
out float fragFade;
out vec3 fragLight;
flat out float fragSeed;
flat out float fragDebris;
void main() {
    // Most of the fading comes in the last third of the stain's life (stains.rs `fade`).
    float age = clamp((now - vertexTexCoord.x) / max(life, 0.001), 0.0, 1.0);
    float late = smoothstep(0.6, 1.0, age);
    float fade = (1.0 - 0.45 * age) * (1.0 - late);
    vec3 p = vertexPosition;
    fragDebris = vertexTexCoord2.x > 8.0 ? 1.0 : 0.0;
    if (fragDebris > 0.5) {
        vec3 up = planet == 1 ? normalize(p) : vec3(0.0, 1.0, 0.0);
        p -= up * vertexTexCoord2.y * (1.0 - fade);
    } else {
        vec3 toEye = viewPosition - p;
        float d = max(length(toEye), 0.001);
        p += toEye / d * (0.03 + 0.002 * d);
    }
    fragWorld = vertexPosition;
    fragNormal = vertexNormal;
    fragColour = vertexColor;
    fragLocal = vertexTexCoord2;
    fragFade = fade;
    // The soot's light, per corner of the tiles under it; debris is lit per fragment.
    fragLight = fragDebris > 0.5 ? vec3(1.0) : terrainLight(vertexPosition, normalize(vertexNormal));
    fragSeed = vertexTexCoord.y;
    gl_Position = mvp * vec4(p, 1.0);
}
