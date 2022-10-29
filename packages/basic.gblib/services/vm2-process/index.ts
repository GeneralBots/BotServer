const crypto2 = require('crypto');
const Fs = require('fs');
const Path = require('path');
const { spawn } = require('child_process');

const {  } = require('child_process');
const { dirname } = require('path');
const { fileURLToPath } = require('url');
const net = require('net');

const genericPool = require('generic-pool');
const finalStream = require('final-stream');

const waitUntil = (condition) => {
  if (condition()) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
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
  limits = Object.assign({
    cpu: 100,
    memory: 2000,
    time: 4000
  }, limits);

  let limitError = null;

  const ref = crypto2.randomBytes(20).toString('hex');

  const kill = (x) => {
    spawn('sh', ['-c', `pkill -9 -f ${ref}`]);
  };

  let stderrCache = '';
  const factory = {
    create: function () {
      
      const runner = spawn('cpulimit', [
        '-ql', limits.cpu,
        '--',
        'node', `--experimental-fetch`, `--max-old-space-size=${limits.memory}`, 
          limits.script
        , ref
      ], { cwd: limits.cwd, shell: false });

      runner.stdout.on('data', (data) => {
        runner.socket = runner.socket || data.toString().trim();
      });

      runner.stderr.on('data', (data) => {
        stderrCache = stderrCache + data.toString();
        if (stderrCache.includes('FATAL ERROR: Reached heap limit Allocation failed - JavaScript heap out of memory')) {
          limitError = 'code execution exceeed allowed memory';
        }
      });

      return runner;
    },

    destroy: function (childProcess) {
      kill(childProcess);
    }
  };

  const pool = genericPool.createPool(factory, { min, max });

  const run = async (code, scope) => {
    const childProcess = await pool.acquire();

    await waitUntil(() => childProcess.socket);

    const socket = net.createConnection(childProcess.socket);

    const timer = setTimeout(() => {
      limitError = 'code execution took too long and was killed';
      kill(childProcess);
    }, limits.time);

    socket.write(JSON.stringify({ code, scope }) + '\n');

    try {
      let data = await finalStream(socket);
      
      data = JSON.parse(data)

      if (data.error) {
        throw new Error(data.error);
      }

      return data.result;
    } catch (error) {
      throw new Error(limitError || error);
    } finally {
      clearTimeout(timer);
      pool.destroy(childProcess);
    }
  };

  return {
    run,
    drain: () => {
      pool.drain().then(() => pool.clear());
    }
  };
};

exports.createVm2Pool = createVm2Pool;
