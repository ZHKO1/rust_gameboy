import { GameBoy, JoyPadKey } from "../pkg/rust_gameboy_wasm.js";
import { memory } from '../pkg/rust_gameboy_wasm_bg';

class Emulator {
  constructor() {
    this.lcd_width = GameBoy.lcd_width();
    this.lcd_height = GameBoy.lcd_height();
    this.gameboy = null;
    this.running = false;
    this.gbc = false;

    this.canvas = document.getElementById("game-of-life-canvas");
    this.ctx = this.canvas.getContext("2d");

    try {
      this.gameboy = new GameBoy();
    } catch (e) {
      console.error(e);
      throw e;
    }
  }

  start() {
    this.gameboy.start();
    this.run();
  }

  load_cartridge(romBuffer) {
    const rom = new Uint8Array(romBuffer);
    try {
      this.gameboy.load_cartridge(rom);
    } catch (e) {
      console.error(e);
      throw e;
    }

    console.log("load_cartridge!");
  }

  is_gbc() {
    try {
      this.gbc = this.gameboy.is_gbc();
      return this.gbc
    } catch (e) {
      console.error(e);
      throw e;
    }
  }

  load_bios(biosBuffer) {
    const biosData = new Uint8Array(biosBuffer);
    try {
      this.gameboy.load_bios(biosData);
    } catch (e) {
      console.error(e);
      throw e;
    }

    console.log("load_bios!");
  }

  run() {
    if (this.frameTimer != null) {
      clearInterval(this.frameTimer);
    }

    this.frameTimer = window.setInterval(() => {
      this.renderFrame()
    }, 16.7504);

    this.running = true;
  }

  renderFrame() {
    const frameBufferPtr = this.gameboy.frame();
    const frameBuffer = new Uint8Array(memory.buffer, frameBufferPtr,
      this.lcd_width * this.lcd_height * 4);
    const imageData = this.ctx.createImageData(this.lcd_width, this.lcd_height);
    const data = imageData.data;

    for (var x = 0; x < this.lcd_width; x += 1) {
      for (var y = 0; y < this.lcd_height; y += 1) {
        const source_idx = y * this.lcd_width * 4 + x * 4;
        const red = frameBuffer[source_idx];
        const green = frameBuffer[source_idx + 1];
        const blue = frameBuffer[source_idx + 2];
        const dest_idx = y * this.lcd_width * 4 + x * 4;
        data[dest_idx] = blue;
        data[dest_idx + 1] = green;
        data[dest_idx + 2] = red;
        data[dest_idx + 3] = 255; // alpha

        // console.log(`${red}${green}${blue}`);
      }
    }
    this.ctx.putImageData(imageData, 0, 0);
  }

  mapKeyCodeToInput(keycode) {
    let joypad_input = null;

    switch (keycode) {
      case "ArrowUp":
        joypad_input = JoyPadKey.Up;
        break;
      case "ArrowDown":
        joypad_input = JoyPadKey.Down;
        break;
      case "ArrowLeft":
        joypad_input = JoyPadKey.Left;
        break;
      case "ArrowRight":
        joypad_input = JoyPadKey.Right;
        break;
      case "KeyZ":
        joypad_input = JoyPadKey.A;
        break;
      case "KeyX":
        joypad_input = JoyPadKey.B;
        break;
      case "Enter":
        joypad_input = JoyPadKey.Start;
        break;
      case "Backspace":
        joypad_input = JoyPadKey.Select;
        break;
      default:
        break;
    }

    return joypad_input;
  }

  handleKey(keyEvent, down) {
    if (this.gameboy == null) {
      return;
    }

    const keyCode = keyEvent.code;
    const joypad_input = this.mapKeyCodeToInput(keyCode);

    if (joypad_input != null && this.running) {
      this.gameboy.input(joypad_input, down);
    }
  }

  save() {
    this.gameboy.quck_save()
  }

  load() {
    this.gameboy.quck_load()
  }
}

const emulator = new Emulator();
async function get_file(path) {
  return fetch(path)
    .then(i => i.arrayBuffer())
}

async function init() {
  load_paintWorklet();
  let rom_promise = await get_file(`assets/pokemon_gold.gbc`);
  await start_game(rom_promise);
}

async function start_game(rom) {
  rom = await Promise.resolve(rom);
  emulator.load_cartridge(rom);
  let bios;
  let type;
  if (emulator.is_gbc()) {
    bios = await get_file(`assets/gbc_bios.bin`);
    type = "GBC";
  } else {
    bios = await get_file(`assets/DMG_ROM.bin`);
    type = "GB";
  }
  emulator.load_bios(bios);
  emulator.start();
  switch_background(type);
}

async function load_paintWorklet() {
  if ("paintWorklet" in CSS) {
    return CSS.paintWorklet.addModule('paintworklet.js')
  }
}

async function switch_background(type) {
  let ele = document.querySelector('.container');
  switch (type) {
    case "GBC":
      ele.classList.add("topurple");
      break;
    case "GB":
      ele.classList.add("togreen");
      break;
  }
  ele.classList.add("animating");
  let start = performance.now();
  requestAnimationFrame(function raf(now) {
    const count = Math.floor(now - start);
    ele.style.cssText = `--animation-tick: ${count};`;
    if (count > 1000) {
      ele.classList.remove("animating");
      ele.style.cssText = `--animation-tick: 0`;
      switch (type) {
        case "GBC":
          ele.classList.remove("green");
          ele.classList.remove("topurple");
          ele.classList.add("purple");
          break;
        case "GB":
          ele.classList.remove("togreen");
          ele.classList.remove("purple");
          ele.classList.add("green");
          break;
      }
      return;
    } else {
      requestAnimationFrame(raf);
    }
  });
}

init();

let start = document.querySelector(".start");
start.addEventListener("click", (event) => {
  let romPicker = document.getElementById("rompicker");
  romPicker.addEventListener("change", async (event) => {
    const romFile = romPicker.files[0];
    let rom_promise = await romFile.arrayBuffer();
    await start_game(rom_promise);
  });
  romPicker.click();
});

let save = document.querySelector(".save");
save.addEventListener("click", (event) => {
  emulator.save();
});

let load = document.querySelector(".load");
load.addEventListener("click", (event) => {
  emulator.load();
});

document.addEventListener("keydown", (event) => {
  emulator.handleKey(event, true);
});

document.addEventListener("keyup", (event) => {
  emulator.handleKey(event, false);
});