# Beam: The Animation Language

Beam is a declarative programming language for creating stunning 2D animations. Inspired by `manim`, Beam provides a simple yet powerful syntax to define scenes, objects, and timelines, which are then compiled by a Rust-based engine into a video file.

## The Beam Language

Beam's syntax is designed to be intuitive and easy to read. You define your animation by declaring scenes, placing objects within them, and then orchestrating their movements and transformations over time in a timeline.

**Key Concepts:**

- **`scene`**: A container for objects and the canvas for your animation.
- **`timeline`**: Defines how object properties change over time.
- **Objects**: Basic shapes like `circle`, `square`, `rectangle`, `line`, `triangle`, `ellipse`, and also `text`.
- **Properties**: Attributes of objects that can be animated, such as `color`, `position`, `scale`, `rotation`, and `opacity`.
- **Time**: Specified in seconds (`s`) or milliseconds (`ms`).

### Example

Here's a simple Beam script that creates a circle and animates its position and opacity.

```beam
scene "MyFirstAnimation" {
    circle "logo" {
        radius: 50,
        color: #00A0D8,
        position: (0, 0)
    }
}

timeline for "MyFirstAnimation" {
    // Animate the logo's position from 0s to 2s
    at 0s to 2s, "logo".position -> (200, 200) with ease_in_out;

    // The logo appears instantly at 2s
    at 2s, "logo".opacity -> 1.0;
}
```

## Features

### Available Shapes

Beam supports a variety of primitive shapes that you can use to build your animations:

- `circle`
- `square`
- `rectangle`
- `line`
- `triangle`
- `ellipse`
- `arrow`
- `double_arrow`
- `vector`

### Animatable Properties

You can animate the following properties of your objects:

- `position`: The (x, y) coordinates of the object.
- `scale`: The size of the object.
- `rotation`: The rotation of the object in degrees.
- `color`: The fill color of the object.
- `border_color`: The border color of the object.
- `opacity`: The transparency of the object, from 0.0 to 1.0.

### Easing Functions

To make your animations feel more natural, you can use these easing functions:

- `ease_in`
- `ease_out`
- `ease_in_out`
- `linear` (default)

### Camera Options

You can configure the global canvas for your animation with these camera properties:

- `width`: The width of the output video in pixels.
- `height`: The height of the output video in pixels.
- `background_color`: The background color of the scene.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)

### Running a Beam Animation

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/your-username/beam.git
    cd beam
    ```

2.  **Build the project:**
    ```bash
    cargo build --release
    ```

3.  **Run the compiler:**
    Provide the path to your Beam script (`.beam` file) as an argument. The output will be a sequence of PNG frames in the `output` directory.

    ```bash
    cargo run --release -- example.beam
    ```

## Development

Interested in contributing to Beam? Here's how you can get started.

### Running Tests

Run the full test suite to ensure everything is working as expected.

```bash
cargo test
```

### Code Coverage

We aim for high code coverage. You can generate a coverage report locally. This requires `cargo-llvm-cov` to be installed.

```bash
# Install if you haven't already
cargo install cargo-llvm-cov

# Run coverage analysis and open the HTML report
cargo llvm-cov --all --open
```

## Releases

Stay tuned for our first official release! 