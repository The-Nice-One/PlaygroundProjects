/* Code.org App Lab library to build declarative User Interfaces.
 * 
 * AUTHOR: Alexis Osorio
 * VERSION: 0.0.3
 * 
 * CREDITS
 * Heavily inspired by IFM´s Sem (id lhyKHRJzz-OUiy81rm2PKYenkYT4MAt2l1AS-FuyOWE)
 * library discussed [here](https://forum.code.org/t/an-applab-library-i-created/37006).
 * 
 * CHANGELOG
 * 0.0.1 - Initial release.
 * 0.0.1 (Duplicate) - Fix self.init(); documentation.
 * 0.0.2 - Fix initialization bugs & self.init(); documentation.
 * 0.0.3 - Fix self.init(); documentation to be acurate.
*/

function $(id) {
  return {
    id: id,
    push: push,
    
    set style(properties) {
      window.__COMPOSE__.StyleStorage.update(this.id, properties);
      setStyle(this.id, toStyleString(properties)); 
    },
    get style() {
      return window.__COMPOSE__.StyleStorage.get(this.id);
    },
    
    set imageURL(url) {setImageURL(this.id, url); },
    get imageURL() {return getImageURL(this.id); }, 
    
    get text() { getText(this.id) },
    set text(string) { setText(this.id, string) },
    
    get checked() { return getChecked(this.id); },
    
    get gx() { return getXPosition(this.id); },
    get gy() { return getYPosition(this.id); },
    set gx(position) { setPosition(this.id, position, this.gy) },
    set gy(position) { setPosition(this.id, this.gx, position) },
    
    get x() { return getProperty(this.id, "x"); },
    get y() { return getProperty(this.id, "y"); },
    set x(position) { setProperty(this.id, "x", position); },
    set y(position) { setProperty(this.id, "y", position); },
    
    get width() { return getProperty(this.id, "width"); },
    get height() { return getProperty(this.id, "height"); },
    set width(dimension) { setProperty(this.id, "width", dimension); },
    set height(dimension) { setProperty(this.id, "height", dimension); },
    
    set click(handler) {onEvent(this.id, "click", function(event) { handler(event); }); },
    set change(handler) {onEvent(this.id, "change", function(event) { handler(event); }); },
    set keyup(handler) {onEvent(this.id, "keyup", function(event) { handler(event); }); },
    set keydown(handler) {onEvent(this.id, "keydown", function(event) { handler(event); }); },
    set keypress(handler) {onEvent(this.id, "keypress", function(event) { handler(event); }); },
    set mousemove(handler) {onEvent(this.id, "mousemove", function(event) { handler(event); }); },
    set mousedown(handler) {onEvent(this.id, "mousedown", function(event) { handler(event); }); },
    set mouseup(handler) {onEvent(this.id, "mouseup", function(event) { handler(event); }); },
    set mouseover(handler) {onEvent(this.id, "mouseover", function(event) { handler(event); }); },
    set mouseout(handler) {onEvent(this.id, "mouseout", function(event) { handler(event); }); },
    set input(handler) {onEvent(this.id, "input", function(event) { handler(event); }); },
  };
}

function push(element) {
  innerHTML(".push", element.compose());
  setParent(element.id, this.id);
}

function append(children) {
  if(children.length === undefined) {
    appendItem(this.children, children);
  } else {
    for(var i = 0; i < children.length; i++) {
      appendItem(this.children, children[i]);
    }
  }
  return this;
}

function compose(hooks) {
  var body = "";
  for(var i = 0; i < this.children.length; i++) {
    body += this.children[i].compose(hooks);
  }
  if(hooks) {
    var id = Object.keys(this.hook)[0];
    hooks[id] = this.hook[id];
  }
  return this.start + body + this.end;
}
function render(id) {
  var hooks = {};
  innerHTML(id, "");
  innerHTML(id, this.compose(hooks) + div(".push").compose());
  runHooks(hooks);
  return this;
}

function runHooks(hooks) {
  var ids = Object.keys(hooks);
  for(var i = 0; i < ids.length; i++) {
    var id = ids[i];
    var hook = hooks[id];
    var properties = Object.keys(hook);
    for (var j = 0; j < properties.length; j++) {
      var property = properties[j];
      var value = hook[property];
      if (property == "style") {
        window.__COMPOSE__.StyleStorage.update(id, value); 
        continue;
      }
      window.__COMPOSE__.HookStorage.get(property)(id, property, value);
    }
  }
}

function toStyleString(value) {
  if (typeof value !== "string") {
    var styleString = "";
    var styleKeys = Object.keys(value);
    for(var j = 0; j < styleKeys.length; j++) {
      var styleKey = styleKeys[j];
      var styleValue = value[styleKey];
      styleString += styleKey + ":" + styleValue + ";";
    }
    value = styleString;
  }
  return value;
}

function generatePropertiesString(properties, hook) {
  if(properties === undefined) { return ""; }
  var keys = Object.keys(properties);
  var propertyString = "";
  for(var i = 0; i < keys.length; i++) {
    var key = keys[i];
    var value = properties[key];
    
    if (hook && (window.__COMPOSE__.HookStorage.get(key) || key === "style")) {
      var id = Object.keys(hook)[0];
      hook[id][key] = value;
      
      if (window.__COMPOSE__.HookStorage.get(key)) continue;
      value = toStyleString(value);
    }
    
    if (key === "__inner__") continue;
    
    propertyString += " " + key + "='" + value + "'";
  }
  return propertyString;
}
function Element(id, start_tag, properties, end_tag) {
  this.id = id;
  this.hook = {};
  this.hook[id] = {};
  this.start = "<" + start_tag + " id='" + id + "'" + generatePropertiesString(properties, this.hook);
  if (end_tag === undefined) {
    this.start += "/>";
    this.end = "";
  } else {
    this.start += ">";
    if (properties && properties.__inner__) {
      this.start += properties.__inner__;
    }
    this.end = "</" + end_tag + ">";
  }
  this.children = [];
  this.append = append;
  this.compose = compose;
  this.render = render;
  this.push = push;
}

function p(id, properties) {
  return new Element(id, "p", properties, "p");
}
function span(id, properties) {
  return new Element(id, "span", properties, "span");
}
function div(id, properties) {
  return new Element(id, "div", properties, "div");
}
function header(id, properties) {
  return new Element(id, "header", properties, "header");
}
function main(id, properties) {
  return new Element(id, "main", properties, "main");
}
function footer(id, properties) {
  return new Element(id, "footer", properties, "footer");
}
function img(id, properties) {
  return new Element(id, "img", properties);
}
function button(id, properties) {
  return new Element(id, "button", properties, "button");
}
function canvas(id, properties) {
  return new Element(id, "canvas", properties, "canvas");
}
function input(id, properties) {
  return new Element(id, "input", properties);
}
function label(id, properties) {
  return new Element(id, "label", properties, "label");
}

var domEvents = [
  "click",
  "change",
  "keyup",
  "keydown",
  "keypress",
  "mousemove",
  "mousedown",
  "mouseup",
  "mouseover",
  "mouseout",
  "input"
];
var domEventHandler = function(id, property, value) {
  onEvent(id, property, value);
};

// Entry point for the Compose Library.
// 
// Call this constructor to initialize the Compose library and return the library as
// an object
//
// Example
// ```js
// var Compose = new ComposeLibrary.init();
// ```
function init() {
  this.p = p;
  this.span = span;
  this.div = div;
  this.header = header;
  this.main = main;
  this.footer = footer;
  this.img = img;
  this.button = button;
  this.canvas = canvas;
  this.input = input;
  this.label = label;
  this.$ = $;
  window.__COMPOSE__ = {
    StyleStorage: {
      map: {},
      update: function(id, properties) {
        var styleKeys = Object.keys(properties);
    
        if (this.map[id] === undefined) this.map[id] = {};
        for (var i = 0; i < styleKeys.length; i++) {
          var key = styleKeys[i];
          var value = properties[key];
          this.map[id][key] = value;
        }
      },
      get: function(id) {
        return this.map[id];
      }
    },
    HookStorage: {
      map: {},
       register: function(property, handler) {
       this.map[property] = handler;
      },
      get: function(property) {
        return this.map[property];
      }
    }
  };
  
  for (var i = 0; i < domEvents.length; i++) {
    var domEvent = domEvents[i];
    window.__COMPOSE__.HookStorage.register(domEvent, domEventHandler);
  }
  window.__COMPOSE__.HookStorage.register("src", function(id, property, value) {
    setImageURL(id, value);
  });
  
}