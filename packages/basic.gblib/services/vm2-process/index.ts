const crypto2 = require('crypto');
const { spawn } = require('child_process');
const CDP = require('chrome-remote-interface');
const {} = require('child_process');
const net = require('net');
import { GBLog } from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBServer } from '../../../../src/app';
import { DebuggerService } from '../DebuggerService';
const finalStream = require('final-stream');

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

const createVm2Pool = ({ min, max, ...limits }) => {
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

  const run = async (code, scope) => {
    const childProcess = spawn(
      'cpulimit',
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
      { cwd: limits.cwd, shell: false }
    );

    childProcess.stdout.on('data', data => {
      childProcess.socket = childProcess.socket || data.toString().trim();
    });

    childProcess.stderr.on('data', data => {
      stderrCache = stderrCache + data.toString();
      if (stderrCache.includes('failed: address already in use')) {
        limitError = stderrCache;
        kill(process);
      }
      if (stderrCache.includes('FATAL ERROR: Reached heap limit Allocation failed - JavaScript heap out of memory')) {
        limitError = 'code execution exceeed allowed memory';
        kill(process);
      }
    });

    let socket = null;
    await waitUntil(() => childProcess.socket);
    socket = net.createConnection(childProcess.socket);
    socket.write(JSON.stringify({ code, scope }) + '\n');

    // Only attach if called by debugger/run.

    if (GBServer.globals.debuggers[limits.botId]) {
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
                  await CollectionUtil.asyncForEach(variables.result, async v => {
                    if (!DebuggerService.systemVariables.filter(x => x === v.name)[0]) {
                      if (v.value.value) {
                        variablesText = `${variablesText} \n ${v.name}: ${v.value.value}`;
                      }
                    }
                  });
                }
                GBLog.info(`BASIC: Breakpoint variables: ${variablesText}`); // (zero-based)

                // Processes breakpoint hits.

                if (hitBreakpoints.length >= 1) {
                  GBLog.info(`BASIC: Break at line ${frame.location.lineNumber + 1}`); // (zero-based)

                  GBServer.globals.debuggers[limits.botId].scope = variablesText;
                  GBServer.globals.debuggers[limits.botId].state = 2;
                } else {
                  GBLog.info(`BASIC: Configuring breakpoints if any for ${limits.botId}`);
                  // Waits for debugger and setup breakpoints.

                  await CollectionUtil.asyncForEach(GBServer.globals.debuggers[limits.botId].breaks, async brk => {
                    try {
                      const { breakpointId } = await client.Debugger.setBreakpoint({
                        location: {
                          scriptId: frame.location.scriptId,
                          lineNumber: brk
                        }
                      });
                      GBLog.info(`BASIC break defined ${breakpointId} for ${limits.botId}`);
                    } catch (error) {
                      GBLog.info(`BASIC error defining defining ${brk} for ${limits.botId}. ${error}`);
                    }
                  });
                  await client.Debugger.resume();
                }
              });

              await client.Runtime.runIfWaitingForDebugger();
              await client.Debugger.enable();
              await client.Runtime.enable();

              resolve(1);
            } catch (err) {
              GBLog.error(err);
              kill(childProcess);
            }
          }).on('error', err => {
            console.error(err);
            kill(childProcess);
            reject(err);
          });
        });
      };

      await debug();
    }

    const timer = setTimeout(() => {
      limitError = 'code execution took too long and was killed';
      kill(childProcess);
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
      clearTimeout(timer);
    }
  };

  return {
    run
  };
};

exports.createVm2Pool = createVm2Pool;
