#version 330
// Grass blades (task 83), after Meshcraft's procedural.fs: the blade's flat normal from the
// screen derivatives, seen from either side, lit by the light that represents the sun as the
// ground is (terrainLight: day, night and the shadow map). The blade normal leans towards the
// ground normal so a meadow keeps the tone of the plain ground it fades into; fading blades
// are lit as the ground is.
in vec3 worldPosition;
in vec3 groundNormal;
in vec4 color;
in float bladeFade;
out vec4 finalColor;
uniform float bladeLight;
/*LIGHTING*/
void main() {
    vec3 normal = normalize(cross(dFdx(worldPosition), dFdy(worldPosition)));
    if (dot(normal, toSun) < 0.0) normal = -normal;
    vec3 lit = normalize(mix(normalize(groundNormal), normal, bladeLight * bladeFade));
    finalColor = vec4(color.rgb * terrainLight(worldPosition, lit), 1.0);
}
