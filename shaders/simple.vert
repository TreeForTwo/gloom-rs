#version 430 core

in layout(location=0) vec3 position;
in layout(location=1) vec3 normal;
in layout(location=2) vec4 colour;

uniform layout(location=3) mat4 transform;
uniform layout(location=4) mat4 model;

out layout(location=1) vec3 outNormal;
out layout(location=2) vec4 outColour;

void main()
{
    gl_Position = transform * vec4(position, 1.0f);
    outNormal = normalize(mat3(model) * normal);
    outColour = colour;
}