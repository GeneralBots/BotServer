import { expect, test } from 'vitest';
import { GBServer } from './src/app';
import { RootData } from './src/RootData';
import { GBMinInstance } from 'botlib-legacy';
import { Mutex } from 'async-mutex';

export default function init() {
  const min = {
    packages: null,
    appPackages: null,
    botId: 'gbtest',
    instance: { botId: 'gbtest' },
    core: {},
    conversationalService: {},
    kbService: {},
    adminService: {},
    deployService: {},
    textServices: {},
    bot: {},
    dialogs: {},
    userState: {},
    userProfile: {},
    whatsAppDirectLine: {},
    cbMap: {},
    scriptMap: {},
    sandBoxMap: {},
    gbappServices: {}
  };

  GBServer.globals = new RootData();
  GBServer.globals.server = null;
  GBServer.globals.httpsServer = null;
  GBServer.globals.webSessions = {};
  GBServer.globals.processes = [0, { pid: 1, proc: { step: {} } }];
  GBServer.globals.files = {};
  GBServer.globals.appPackages = [];
  GBServer.globals.sysPackages = [];
  GBServer.globals.minInstances = [min];
  GBServer.globals.minBoot = min;
  GBServer.globals.wwwroot = null;
  GBServer.globals.entryPointDialog = null;
  GBServer.globals.debuggers = [];
  GBServer.globals.indexSemaphore = new Mutex();
  GBServer.globals.users = { 1: { userId: 1 } };
}
