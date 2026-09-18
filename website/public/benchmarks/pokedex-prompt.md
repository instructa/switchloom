Implement this benchmark now in the current directory. Use the attached reference image for the physical Pokédex appearance. Work only in this project directory; do not inspect sibling benchmarks, previous implementations, or review reports. Scaffold, implement, and verify the complete application. Do not publish or deploy it.

# PRODUCT SPECIFICATION

Build a production-quality interactive web recreation of the classic Kanto-era red Pokédex.

It should immediately read visually as the iconic physical Pokédex device rather than as a conventional website placed inside a red rectangle.

This is a 2D web interface. Do not use Three.js, WebGL, Canvas rendering, a 3D engine, or 3D models.

Use React + TypeScript + Vite. If the directory is empty, scaffold the project yourself.

Use normal CSS for the device and interface. Do not use Tailwind, Bootstrap, Material UI, shadcn, or another UI/component framework. Small utility packages are acceptable when genuinely useful.

Use PokéAPI REST v2 as the public data source.

No backend and no authentication.

## Pokémon scope

The Pokédex covers the original Generation I set only:

\#001 Bulbasaur through #151 Mew.

Load #001 Bulbasaur by default.

Fetch the Pokémon list efficiently and load detailed Pokémon data lazily rather than eagerly downloading every detail endpoint.

Cache fetched API resources locally so repeated navigation does not make unnecessary requests.

Use Pokémon sprites supplied through PokéAPI. Do not download unrelated external artwork.

## Pokédex appearance

Create the complete Pokédex casing in CSS.

The desktop layout should resemble an opened red Pokédex with two physical halves connected by a central hinge.

Important recognizable elements:

- deep red molded outer casing
- darker red edges and panel seams
- central vertical hinge
- large circular blue lens/light in the upper-left area with a glass-like highlight
- three small red/yellow/green indicator lamps
- inset main display with a substantial light-colored bezel
- Pokémon sprite displayed inside the main screen
- physical-looking round button
- directional D-pad
- speaker/details such as small slots or vents
- right-side information LCD
- recognizable blue rectangular keypad/button grid
- smaller yellow/orange/black control buttons
- device labels and typography appropriate to a retro electronic encyclopedia

Use CSS borders, gradients, inset shadows, highlights, and subtle depth to make the casing convincing, but keep the layout fundamentally 2D. Do not use CSS 3D perspective transforms.

Avoid excessive modern glassmorphism.

The screens should have a restrained retro LCD/electronic feel.

The casing is the visual hero. It should feel carefully constructed rather than like a dashboard template.

Do not simply display a grid of Pokémon cards.

## Main Pokémon display

For the selected Pokémon show:

- National Pokédex number with three digits
- properly formatted name
- front sprite
- type or types
- height
- weight
- abilities
- English Pokédex description
- base stats

Use the appropriate PokéAPI Pokémon and species data.

Clean newline/form-feed artifacts from flavor text.

Provide compact views/tabs such as DATA and STATS if necessary to fit information naturally into the physical device screens.

## Navigation

The Pokédex must be genuinely interactive.

Support:

- Previous Pokémon
- Next Pokémon
- clickable physical controls
- D-pad navigation
- keyboard left/right navigation
- search by Pokémon name
- search by Pokédex number
- direct selection from a compact list/index of the 151 Pokémon

Searching `pikachu`, `Pikachu`, `25`, or `025` must resolve to Pikachu.

Do not navigate below #001 or above #151.

Controls should visibly react when pressed.

The currently selected Pokémon must be visually obvious.

## Data states

Implement deliberate UI states for:

- initial loading
- navigation loading
- API error
- invalid search
- successful cached navigation

Do not replace the entire Pokédex with a generic spinner while switching Pokémon. Keep the physical device visible.

An API failure must not crash the application.

## Responsive design

Desktop is the primary showcase.

Target the main showcase around a 1440×900 viewport.

The Pokédex should remain usable on a phone-sized viewport around 390×844.

For narrow screens, intelligently stack/fold/rearrange the two device halves while preserving the Pokédex identity.

Do not merely shrink the desktop UI until text becomes unreadable.

## Quality details

Add tasteful micro-interactions:

- button press states
- indicator changes
- subtle screen transitions
- focus states
- hover states where appropriate

Respect prefers-reduced-motion.

Use semantic buttons and inputs and provide accessible names for non-textual controls.

Keep the code structured and typed. Avoid one enormous React component.

## API behavior

Use the official PokéAPI v2 REST endpoints.

Handle network requests through a small typed API/data layer rather than scattering fetch calls throughout components.

Implement local caching.

Do not hammer the API.

## Tests and verification

At minimum verify:

1. application builds successfully
2. \#001 Bulbasaur loads by default
3. next/previous navigation works and respects 1/151 boundaries
4. keyboard navigation works
5. searching `pikachu` works
6. searching `25` and `025` resolves to #025
7. invalid searches produce a useful state
8. Pokémon data is rendered from API responses
9. repeated access can use the local cache
10. API errors do not crash the app

Add focused automated tests where useful.

Run type checking, tests, and production build before declaring completion.

If browser automation is available, also perform a real browser smoke test at desktop and mobile viewport sizes.

If screenshots can genuinely be generated from the running application, save:

artifacts/pokedex-desktop.png
artifacts/pokedex-mobile.png

Do not create fake screenshots.

## Benchmark constraints

Do not add AI features to the Pokédex.

Do not add unrelated features just to increase implementation size.

Do not replace the requested physical Pokédex design with a generic modern Pokédex website.

Do not display which model built the application anywhere in the app.

Prioritize in this order:

1. visual fidelity and polish
2. correct interaction
3. reliable PokéAPI integration
4. responsive behavior
5. maintainable implementation

The finished result should be impressive enough that a screenshot alone makes someone want to open the demo.

When complete, give me a concise report containing:

- what was built
- verification commands and results
- any known limitations
