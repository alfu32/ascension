#version 330
// The sea. Far out: the map colour lit from the radial normal with a soft highlight. Up close:
// the water program (after meshcraft's water.fs) — the shallow and deep materials blended by
// depth, moving ripples, sky reflection by Fresnel, a sun glint and foam along the coast.
// The vertex colour is the map colour shaded by fog of war; its alpha encodes the depth.
in vec4 fragColor;
in vec3 fragPosition;
uniform vec3 viewPosition;
uniform float detail;
uniform vec2 detailRange;
uniform vec3 shallowColour;
uniform vec3 shallowAlt;
uniform vec3 deepColour;
uniform vec3 deepAlt;
out vec4 finalColor;
/*LIGHTING*/
/*NOISE*/
uniform float time;
const vec3 MAP_SHALLOW = vec3(0.22, 0.62, 0.74);
const vec3 MAP_DEEP = vec3(0.03, 0.14, 0.38);
void main() {
    vec3 up = planet == 1 ? normalize(fragPosition) : vec3(0.0, 1.0, 0.0);
    vec3 toEye = normalize(viewPosition - fragPosition);
    float day = daylight(fragPosition);
    float depth = clamp((fragColor.a - 0.35) / 0.5, 0.0, 1.0);
    float px = length(fwidth(fragPosition));
    float near = detail * (1.0 - smoothstep(detailRange.x, detailRange.y, distance(viewPosition, fragPosition)));

    // Ripples as a moving height field; they flatten where a pixel spans several waves.
    float waves = valueNoise(fragPosition * 1.3 + vec3(time * 0.32, time * 0.07, time * 0.21)) * 0.6
                + valueNoise(fragPosition * 3.7 - vec3(time * 0.25, time * 0.33, -time * 0.12)) * 0.4;
    float ripple = waves * 0.10 * keep(3.7, px) * near;
    vec3 normal = patternBump(up, ripple, fragPosition);

    float diffuse = max(dot(normal, toSun), 0.0);
    float lit = diffuse > 0.0 ? visibility(fragPosition, normal) : 1.0;
    vec3 halfVector = toSun + toEye;
    float halfCosine = dot(normal, halfVector / max(length(halfVector), 0.00001));
    float glintPower = mix(64.0, 220.0, near);
    float glint = pow(max(halfCosine, 0.0), glintPower) * diffuse * lit * day;
    vec3 light = mix(NIGHT_AMBIENT, vec3(DAY_AMBIENT + DIRECT * diffuse * lit), day);

    vec3 far = fragColor.rgb * light + vec3(0.30, 0.34, 0.38) * glint;

    vec3 shade = clamp(fragColor.rgb / max(mix(MAP_SHALLOW, MAP_DEEP, depth), vec3(0.01)), 0.0, 2.0);
    vec3 shallow = mix(shallowColour, shallowAlt, waves);
    vec3 deep = mix(deepColour, deepAlt, waves);
    vec3 water = mix(shallow, deep, smoothstep(0.0, 0.7, depth));
    float facing = max(dot(normal, toEye), 0.0);
    float fresnel = 0.02 + 0.98 * pow(1.0 - facing, 5.0);
    vec3 sky = mix(vec3(0.36, 0.56, 0.80), vec3(0.62, 0.78, 0.95), facing) * day + NIGHT_AMBIENT;
    float coast = 1.0 - smoothstep(0.0, 0.12, depth);
    float foam = coast * smoothstep(0.45, 0.85, valueNoise(fragPosition * 2.4 + vec3(time * 0.4)))
               * (0.6 + 0.4 * sin(time * 1.3 + depth * 40.0));
    vec3 surface = mix(water * light, sky, fresnel * 0.35) * shade + vec3(0.85, 0.82, 0.7) * glint * 0.7;
    surface = mix(surface, vec3(0.92, 0.96, 1.0) * light * shade, foam * 0.7);
    float alpha = mix(fragColor.a, clamp(0.5 + 0.45 * depth + foam * 0.3 + fresnel * 0.2, 0.0, 1.0), near);

    finalColor = vec4(mix(far, surface, near), min(alpha + glint * 0.4, 1.0));
}
