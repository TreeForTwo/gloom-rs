#version 430 core

in layout(location=0) vec3 position;
in layout(location=1) vec4 colour;

uniform layout(location=2) float oscillation;

out layout(location=1) vec4 outColour;

void main()
{
    float x = oscillation;
    mat4 transform = {{1, 0, 0, 0},
                      {x, 1, 0, 0},
                      {0, 0, 1, 0},
                      {0, 0, 0, 1}};
    gl_Position = transform * vec4(position, 1.0f);
    outColour = colour;
}