import crypto2 from 'crypto';
import { spawn } from 'child_process';
import CDP from 'chrome-remote-interface';
import {} from 'child_process';
import net from 'net';
import { GBLog } from 'botlib-legacy';

import { GBServer } from '../../../../src/app.js';
import { DebuggerService } from '../DebuggerService.js';
import { GBLogEx } from '../../../core.gbapp/services/GBLogEx.js';
import { GBUtil } from '../../../../src/util.js';

let finalStream: any = null;
try {
  finalStream = await import('final-stream');
} catch {}

const waitUntil = condition => {
  if (condition()) {
    return Promise.resolve();
  }

  return new Promise(resolve => {
    const interval = setInterval(() => {
      if (!condition()) {
        return;
      }

      clearInterval(interval);
      resolve(0);
    }, 0);
  });
};

const systemVariables = [
  'AggregateError',
  'Array',
  'ArrayBuffer',
  'Atomics',
  'BigInt',
  'BigInt64Array',
  'BigUint64Array',
  'Boolean',
  'DataView',
  'Date',
  'Error',
  'EvalError',
  'FinalizationRegistry',
  'Float32Array',
  'Float64Array',
  'Function',
  'Headers',
  'Infinity',
  'Int16Array',
  'Int32Array',
  'Int8Array',
  'Intl',
  'JSON',
  'Map',
  'Math',
  'NaN',
  'Number',
  'Object',
  'Promise',
  'Proxy',
  'RangeError',
  'ReferenceError',
  'Reflect',
  'RegExp',
  'Request',
  'Response',
  'Set',
  'SharedArrayBuffer',
  'String',
  'Symbol',
  'SyntaxError',
  'TypeError',
  'URIError',
  'Uint16Array',
  'Uint32Array',
  'Uint8Array',
  'Uint8ClampedArray',
  'VM2_INTERNAL_STATE_DO_NOT_USE_OR_PROGRAM_WILL_FAIL',
  'WeakMap',
  'WeakRef',
  'WeakSet',
  'WebAssembly',
  '__defineGetter__',
  '__defineSetter__',
  '__lookupGetter__',
  '__lookupSetter__',
  '__proto__',
  'clearImmediate',
  'clearInterval',
  'clearTimeout',
  'console',
  'constructor',
  'decodeURI',
  'decodeURIComponent',
  'dss',
  'encodeURI',
  'encodeURIComponent',
  'escape',
  'eval',
  'fetch',
  'global',
  'globalThis',
  'hasOwnProperty',
  'isFinite',
  'isNaN',
  'isPrototypeOf',
  'parseFloat',
  'parseInt',
  'process',
  'propertyIsEnumerable',
  'setImmediate',
  'setInterval',
  'setTimeout',
  'toLocaleString',
  'toString',
  'undefined',
  'unescape',
  'valueOf'
];

export const createVm2Pool = ({ min, max, ...limits }) => {
  limits = Object.assign(
    {
      cpu: 100,
      memory: 2000,
      time: 4000
    },
    limits
  );

  let limitError = null;

  const ref = crypto2.randomBytes(20).toString('hex');

  const kill = x => {
    spawn('sh', ['-c', `pkill -9 -f ${ref}`]);
  };

  let stderrCache = '';

  const run = async (code: any, scope: any) => {
    // Configure environment variables
    const env = Object.assign({}, process.env, {
      NODE_ENV: 'production',
      NODE_OPTIONS: '' // Clear NODE_OPTIONS if needed
    });

    const childProcess = spawn(
      '/usr/bin/cpulimit',
      [
        '-ql',
        limits.cpu,
        '--',
        'node',
        `${limits.debug ? '--inspect=' + limits.debuggerPort : ''}`,
        `--experimental-fetch`,
        `--max-old-space-size=${limits.memory}`,
        limits.script,
        ref
      ],
      { cwd: limits.cwd, shell: true, env: env }
    );

    childProcess.stdout.on('data', data => {
      childProcess['socket'] = childProcess['socket'] || data.toString().trim();
    });

    childProcess.stderr.on('data', data => {
      stderrCache = stderrCache + data.toString();
      if (stderrCache.includes('failed: address already in use')) {
        limitError = stderrCache;
        kill(process);
        GBServer.globals.debuggers[limits.botId].state = 0;
        GBServer.globals.debuggers[limits.botId].stateInfo = stderrCache;
      } else if (
        stderrCache.includes('FATAL ERROR: Reached heap limit Allocation failed - JavaScript heap out of memory')
      ) {
        limitError = 'code execution exceeed allowed memory';
        kill(process);
        GBServer.globals.debuggers[limits.botId].state = 0;
        GBServer.globals.debuggers[limits.botId].stateInfo = 'Fail';
      } else if (stderrCache.includes('Debugger attached.')) {
        GBLogEx.info(min, `General Bots Debugger attached to Node .gbdialog process for ${limits.botId}.`);
      }
    });

    let socket = null;
    await waitUntil(() => childProcess['socket']);

    GBServer.globals.debuggers[limits.botId].childProcess = ref;

    // Only attach if called by debugger/run.

    if (limits.debug) {
      const debug = async () => {
        return new Promise((resolve, reject) => {
          CDP(async client => {
            const { Debugger, Runtime } = client;
            try {
              GBServer.globals.debuggers[limits.botId].client = client;

              await client.Debugger.paused(async ({ callFrames, reason, hitBreakpoints }) => {
                const frame = callFrames[0];

                // Build variable list ignoring system variables of script.

                const scopeObjectId = frame.scopeChain[2].object.objectId;
                const variables = await Runtime.getProperties({ objectId: scopeObjectId });
                let variablesText = '';
                if (variables && variables.result) {
                  await GBUtil.asyncForEach(variables.result, async v => {
                    if (!systemVariables.filter(x => x === v.name)[0]) {
                      if (v.value.value) {
                        variablesText = `${variablesText} \n ${v.name}: ${v.value.value}`;
                      }
                    }
                  });
                }
                GBServer.globals.debuggers[limits.botId].scope = variablesText;
                GBLogEx.info(min, `Breakpoint variables: ${variablesText}`); // (zero-based)
                // Processes breakpoint hits.

                if (hitBreakpoints.length >= 1) {
                  GBLogEx.info(min, `Break at line ${frame.location.lineNumber + 1}`); // (zero-based)

                  GBServer.globals.debuggers[limits.botId].state = 2;
                  GBServer.globals.debuggers[limits.botId].stateInfo = 'Break';
                } else {
                  GBLog.verbose(`Configuring breakpoints if any for ${limits.botId}...`);
                  // Waits for debugger and setup breakpoints.

                  await GBUtil.asyncForEach(GBServer.globals.debuggers[limits.botId].breaks, async brk => {
                    try {
                      const { breakpointId } = await client.Debugger.setBreakpoint({
                        location: {
                          scriptId: frame.location.scriptId,
                          lineNumber: brk
                        }
                      });
                      GBLogEx.info(min, `BASIC break defined ${breakpointId} for ${limits.botId}`);
                    } catch (error) {
                      GBLogEx.info(min, `BASIC error defining ${brk} for ${limits.botId}. ${error}`);
                    }
                  });
                  await client.Debugger.resume();
                }
              });

              await client.Runtime.runIfWaitingForDebugger();
              await client.Debugger.enable();
              await client.Runtime.enable();

              resolve(1);
            } catch (error) {
              GBLog.error(error);
              kill(childProcess);
              GBServer.globals.debuggers[limits.botId].state = 0;
              GBServer.globals.debuggers[limits.botId].stateInfo = 'Stopped';
            }
          }).on('error', err => {
            console.error(err);
            kill(childProcess);
            GBServer.globals.debuggers[limits.botId].state = 0;
            GBServer.globals.debuggers[limits.botId].stateInfo = 'Stopped';
            reject(err);
          });
        });
      };

      await debug();
    }
    socket = net.createConnection(childProcess['socket']);
    socket.write(JSON.stringify({ code, scope }) + '\n');

    const timer = setTimeout(() => {
      limitError = 'code execution took too long and was killed';

      kill(childProcess);
      GBServer.globals.debuggers[limits.botId].state = 0;
      GBServer.globals.debuggers[limits.botId].stateInfo = limitError;
    }, limits.time);

    try {
      let data = await finalStream(socket);

      data = JSON.parse(data);

      if (!data.length) {
        return null;
      }
      if (data.error) {
        throw new Error(data.error);
      }

      return data.result;
    } catch (error) {
      throw new Error(limitError || error);
    } finally {
      kill(childProcess);

      GBServer.globals.debuggers[limits.botId].state = 0;
      GBServer.globals.debuggers[limits.botId].stateInfo = 'Stopped';
      clearTimeout(timer);
    }
  };

  return {
    run
  };
};
