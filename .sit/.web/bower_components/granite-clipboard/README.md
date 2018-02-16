# granite-clipboard

> A is a lightweight element wrapping-up [clipboard.js](https://clipboardjs.com/) 
> 
> Hybrid Polymer element, 1.x-2.x ready


## Doc & demo

[https://lostinbrittany.github.io/granite-clipboard](https://lostinbrittany.github.io/granite-clipboard)


## Usage example

<!--
```
<custom-element-demo>
  <template>
    <script src="../webcomponentsjs/webcomponents-lite.js"></script>
    <link rel="import" href="granite-clipboard.html">
    <next-code-block></next-code-block>
  </template>
</custom-element-demo>
```
-->
```html
<granite-clipboard text="Text to be copied">
    <div style="width:250px;height:50px;border-radius:5px;background-color:#aaaaff;cursor:pointer;">Click here</div>
</granite-clipboard> 
```



## Install

Install the component using [Bower](http://bower.io/):

```sh
$ bower install LostInBrittany/granite-clipboard --save
```

Or [download as ZIP](https://github.com/LostInBrittany/granite-clipboard/archive/gh-pages.zip).## Usage

1. Import Web Components' polyfill (if needed):

    ```html
    <script src="bower_components/webcomponentsjs/webcomponents.js"></script>
    ```

2. Import Custom Element:

    ```html
    <link rel="import" href="bower_components/granite-clipboard/granite-clipboard.html">
    ```

3. Start using it!

    ```html
    <granite-clipboard level="danger">This is a serious alert!</granite-clipboard>
    ```


## Attributes

Attribute     | Type      | Default  | Description
---           | ---       | ---      | ---
`text`        | *String*  | ``       | The text to copy
`action`      | *String*  | `copy`   | The action to do (either `copy` or `cut`)  



## Contributing

1. Fork it!
2. Create your feature branch: `git checkout -b my-new-feature`
3. Commit your changes: `git commit -m 'Add some feature'`
4. Push to the branch: `git push origin my-new-feature`
5. Submit a pull request :D

## License

[MIT License](http://opensource.org/licenses/MIT)