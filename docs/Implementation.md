# Sim;X Implementation

## Стратегия разработки

Проект должен идти по принципу **спецификация вперед кода**.

Перед написанием Rust-модулей нужно подготовить подробные Markdown-спецификации. Эти спецификации должны быть достаточно строгими, чтобы по ним можно было генерировать код с помощью Codex или другой AI-модели без потери архитектурных правил.

Не нужно начинать сразу со всех четырех доменов, внешнего extension runtime и
community-инфраструктуры. Первый этап — один полностью рабочий вертикальный
срез.

## Первый вертикальный срез

Рекомендуемый первый срез:

**Sim;Phys -> Phys;Mechanics**

Минимальный набор возможностей:

- тело с типизированной массой
- приложенная и результирующая сила
- ускорение по `F = m * a`
- детерминированная интеграция скорости и положения
- renderer-neutral snapshot для редактора
- маятник как первая композиция тела, ограничения и сил

Цели первого среза:

- получить рабочий и показываемый продукт на раннем этапе
- проверить `SimContext` на практике
- проверить типы единиц измерения
- проверить реестр констант
- проверить защиту от NaN и переполнений
- проверить UI для изменения параметров
- проверить разделение editor state, canonical mechanics state и visual state
- получить паттерн, который потом можно переносить в остальные домены

## Железные правила реализации

Любая сгенерированная или написанная вручную реализация обязана соблюдать эти правила.

### 1. Никаких хардкоженных фундаментальных констант

Запрещено:

```rust
let c = 299_792_458.0;
let gamma = 1.0 / (1.0 - v * v / (c * c)).sqrt();
```

Разрешено:

```rust
let c = ctx.constants().c;
let gamma = 1.0 / (1.0 - v * v / (c.0 * c.0)).sqrt();
```

Исключения возможны только для безразмерных математических коэффициентов, которые являются частью самой формулы, например `0.5` в `E = 0.5 * m * v^2`.

### 2. Единая система единиц

Базовая система - СИ.

Каждая функция должна либо использовать типизированные единицы:

```rust
pub fn kinetic_energy(mass: Kilograms, speed: MetersPerSecond) -> Joules;
```

либо явно документировать единицы:

```rust
/// mass_kg: kilograms
/// speed_m_s: meters per second
/// returns: joules
pub fn kinetic_energy(mass_kg: f64, speed_m_s: f64) -> f64;
```

Нельзя допускать ситуацию, где один модуль считает расстояние в метрах, другой в пикселях, а третий в условных единицах без явного преобразования.

### 3. Версионированный SimContext

`SimContext` должен иметь явную версию API.

```rust
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
```

Правила:

- breaking change требует bump `major`
- совместимое добавление требует bump `minor`
- исправление без изменения API требует bump `patch`
- persisted scene, Custom Object и domain schema должны декларировать
  ожидаемую версию
- несовместимость версий должна давать явную ошибку или проходить через
  документированную migration

### 4. Строгий формат Markdown-спек

Спеки должны писаться по единому шаблону, чтобы разные AI-модели интерпретировали их одинаково.

Обязательные секции для функции:

````markdown
## Function: function_name

### Signature

```rust
pub fn function_name(arg: ArgType) -> ReturnType;
```

### Units

- `arg`: unit description
- returns: unit description

### Invariants

- invariant 1
- invariant 2

### Side Effects

- side effect 1
- none

### Errors / Clamps

- error or clamp rule 1
- error or clamp rule 2

### Reads Constants

- `ctx.constants.c`
- none

### Notes

Additional implementation notes.
````

Обязательные секции для модуля:

````markdown
# Module: module_name

## Purpose

## Public Types

## Public Functions

## SimContext Usage

## Constants Usage

## Units

## Invariants

## Error Handling

## Tests Required
````

## Численная безопасность

Каждый шаг симуляции должен защищаться от некорректных чисел.

Обязательные проверки:

- нет NaN в позиции
- нет NaN в скорости
- нет NaN в силе
- нет infinity в состоянии сущности
- масса не отрицательная
- радиус не отрицательный
- `dt` положительный и конечный
- энергия, температура и давление не уходят в недопустимые значения без явного clamp/error

Пример для релятивистских формул:

```rust
let c = ctx.constants().c.0;
let beta_squared = (v * v) / (c * c);

if !beta_squared.is_finite() || beta_squared >= 1.0 {
    return RelativityResult::Clamped;
}

let gamma = 1.0 / (1.0 - beta_squared).sqrt();
```

При экстремальных пользовательских константах симуляция может стать странной, но состояние приложения не должно разрушаться.

## Начальные модули

Точная карта каталогов и направлений зависимостей находится в `Structure.md`.
Ни один домен не должен импортировать соседний домен, а `sim_engine` разрешен
только внутри `src/presentation/sim_engine/`.

### foundation

Общий фундамент:

- constants
- units
- stable identifiers
- versioned domain contracts
- numeric_safety
- api_version

`foundation` не должен превращаться в `common` или `utils`: физические силы,
химические реакции, биологические клетки и renderer-типы сюда не входят.

### domains/phys

Физический фундамент:

- mechanics
- body
- force
- constraint
- integrator
- renderer-neutral snapshot
- thermodynamics
- particle
- collision
- effects
- sandbox

Первый вертикальный срез размещается отдельно в
`domains/phys/mechanics`. Остальные домены живут в собственных каталогах:
`domains/math`, `domains/biol` и `domains/chem`.

### presentation

Интерфейс:

- выбор домена
- выбор subdomain
- полноэкранный Mechanics editor
- palette, canvas и inspector
- панель параметров
- редактор констант
- кнопка reset-to-real
- отображение предупреждений о clamp/error

Интеграция с Sim;Engine изолируется в `presentation/sim_engine`. Этот адаптер
преобразует готовые domain read models в `Scene`, particle/scalar-field ресурсы
или retained 3D state. Он не выполняет simulation step и не хранит каноническое
состояние домена.

### Extensibility

Активного plugin runtime нет. Политика зафиксирована в `EXTENSIBILITY.md`:

- пользовательские расширения из существующих правил сохраняются как Custom
  Objects и recipes;
- новое официальное научное поведение реализуется first-party Rust rule packs
  внутри владеющего domain и компилируется вместе с Sim;X;
- Lua bindings и native dynamic Rust plugin ABI не проектируются;
- возможные sandboxed executable extensions откладываются до появления
  конкретного сценария, который нельзя выразить первыми двумя уровнями.

Публичные domain/application ports нельзя обобщать ради гипотетического
runtime. First-party rule pack использует те же typed commands, capabilities,
relationships, validation и atomic commit, что и основной domain-код.

## План запуска

### Этап 1: документация

- создать базовые проектные Markdown-файлы
- описать общую архитектуру
- описать шаблон спеки
- описать первый вертикальный срез

### Этап 2: общий фундамент

- ввести `Constants`
- ввести typed units или строгую документацию единиц
- ввести `EntityId`
- ввести базовую сущность
- ввести `SimContext`
- ввести numeric safety helpers

### Этап 3: Sim;Phys;Mechanics

До реализации этого этапа обязательны границы и implementation gates из
`COMPOSITION.md`. Если численный метод, capability, relationship или порядок
interaction rules там остаётся открытым, решение сначала фиксируется в
спецификации и только потом становится Rust-кодом.

- typed units для массы, силы, времени, ускорения, скорости и положения
- canonical body state и stable identity
- накопление результирующей силы независимо от visual arrows
- ускорение по `force / mass`
- явно выбранный и документированный integrator
- atomic validate-then-commit simulation step
- bounded immutable presentation snapshot
- маятник как composition поверх mechanics contracts

### Этап 4: UI

- dark borderless full-screen shell
- навигация `Sim;X -> Sim;Phys -> Phys;Mechanics`
- семь Phys subdomains, включая Phys;Sandbox
- smooth hover на обоих уровнях
- realtime viewport
- Mechanics editor с build palette, canvas и inspector
- панель параметров симуляции
- редактор констант
- reset-to-real
- warnings/clamps display

### Этап 5: проверка архитектуры

- добавить Gravity effect и constraint capability
- собрать маятник из body, anchor, constraint и применимых сил
- убедиться, что редактор отправляет intents, а не меняет mechanics state
- убедиться, что mechanics не зависит от UI или Sim;Engine
- убедиться, что Sim;Engine adapter только отображает immutable snapshot

### Этап 6: Saved Compositions и Rule Packs

- стабилизировать versioned graph format после Mechanics composition gates
- сохранять Custom Object как подграф с relationships и local IDs
- транзакционно remap-ить IDs при создании экземпляра
- добавить первый встроенный объект через тот же recipe path
- оформлять новое официальное поведение как first-party Rust rule pack
- не выбирать внешний extension runtime без отдельной утверждённой спеки

## Минимальные тесты

На раннем этапе нужны тесты не только на математику, но и на архитектурные запреты.

Проверить:

- `Constants::real_world()` возвращает конечные положительные значения для физических констант
- reset-to-real восстанавливает реальные значения
- `dt <= 0` не принимается
- сущность с NaN-позицией не попадает в состояние мира
- сила с NaN не применяется
- нулевая или отрицательная масса не принимается
- для одинаковой силы удвоение массы вдвое уменьшает ускорение
- неудачный mechanics step не изменяет состояние частично
- модуль не использует хардкоженную фундаментальную константу там, где должен читать `ctx.constants`
- `EffectProvider::applies_to` не зависит от конкретной подкатегории, если достаточно компонентов сущности
- Custom Object с неподдерживаемой schema version не создаёт частичный instance
- first-party rule pack не обходит domain validation и deterministic schedule

## Главный критерий успеха первого среза

Первый срез считается успешным, когда можно открыть полноэкранное приложение,
выбрать `Sim;Phys -> Phys;Mechanics`, создать или выбрать тело, изменить массу и
силу, увидеть ускорение и движение согласно `F = m * a`, собрать маятник из
mechanics-компонентов, использовать pause/slow-motion/fast-motion/reset и не получить NaN,
infinity, частичную мутацию или crash даже при экстремальных вводах.
