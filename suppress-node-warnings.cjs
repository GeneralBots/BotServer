// inspired by 
// https://github.com/nodejs/node/issues/30810#issuecomment-1383184769
const { emit: originalEmit } = process;

function suppresser(event, error) {
  return event === 'warning' && error.name === 'ExperimentalWarning'
    ? false
    : originalEmit.apply(process, arguments);
}

process.emit = suppresser;