[![Published on webcomponents.org](https://img.shields.io/badge/webcomponents.org-published-blue.svg)](https://www.webcomponents.org/element/jifalops/from-now)

# from-now
A polymer element that strategically updates its output based on the age of
`time`. The default output is the relative time (e.g. 8 hours ago).

## Installation

```
bower i -S jifalops/from-now        # Polymer 2.0 class based
bower i -S jifalops/from-now#0.4.0  # Polymer 2.0 hybrid (1.x compatible)
bower i -S jifalops/from-now#0.3.0  # Polymer 1.x based
```

## Usage
Simply give it a timestamp including milliseconds and it will output time
relative to now in a human friendly format. It will also update the output
according to how far away it is (e.g. once per day/hour/minute).

## Demo
<!--
```
<custom-element-demo>
  <template>
    <script src="../webcomponentsjs/webcomponents-lite.js"></script>
    <link rel="import" href="from-now.html">
    <next-code-block></next-code-block>
    <script>
      document.getElementById('now1').time = Date.now();
      document.getElementById('now2').time = Date.now();
      document.getElementById('now3').time = Date.now();
      document.getElementById('now4').time = Date.now();
    </script>
  </template>
</custom-element-demo>
```
-->

```html
<from-now id="now1"></from-now> | default<br/>
<from-now id="now2" idle></from-now> | idle<br/>
<from-now id="now3" use-absolute></from-now> | absolute<br/>
<from-now id="now4" use-absolute format="YYYY-MM-DD HH:mm"></from-now> | formatted absolute<br/>
(mouseover for alternate format)
```

Full demo:
[webcomponents.org](https://www.webcomponents.org/element/jifalops/from-now/demo/demo/index.html)
| [github](https://jifalops.github.io/from-now/components/from-now/demo/)

API: [webcomponents.org](https://www.webcomponents.org/element/jifalops/from-now/from-now)

## Contributing

1. Fork it on Github.
2. Create your feature branch: `git checkout -b my-new-feature`
3. Commit your changes: `git commit -am 'Add some feature'`
4. Push to the branch: `git push origin my-new-feature`
5. Submit a pull request

## License

[MIT](https://opensource.org/licenses/MIT)
