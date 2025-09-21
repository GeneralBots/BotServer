import { ConversationReference } from 'botbuilder';
import { GBLog, GBMinInstance, GBService, IGBInstance } from 'botlib-legacy';

import { GuaribasUser } from '../models/index.js';
import { FindOptions } from 'sequelize';
import { DialogKeywords } from '../../../packages/basic.gblib/services/DialogKeywords.js';
import fs from 'fs/promises';
import mkdirp from 'mkdirp';
import urlJoin from 'url-join';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBServer } from '../../../src/app.js';
import { GBUtil } from '../../../src/util.js';

/**
 * Security service layer.
 */
export class SecService extends GBService {
  public async ensureUser(
    min: GBMinInstance,
    userSystemId: string,
    userName: string,
    address: string,
    channelName: string,
    displayName: string,
    email: string
  ): Promise<GuaribasUser> {
    const gbaiPath = GBUtil.getGBAIPath(min.botId);
    const dir = urlJoin('work', gbaiPath, 'users', userSystemId);

    if (!(await GBUtil.exists(dir))) {
      mkdirp.sync(dir);
    }

    let user = await GuaribasUser.findOne({
      where: {
        userSystemId: userSystemId
      }
    });

    if (!user) {
      user = GuaribasUser.build();
    }

    const systemPromptFile = urlJoin(dir, 'systemPrompt.txt');
    if (await GBUtil.exists(systemPromptFile)) {
      user['systemPrompt'] = await fs.readFile(systemPromptFile);
    }

    user.instanceId = min.instance.instanceId;
    user.userSystemId = userSystemId;
    user.userName = userName;
    user.displayName = displayName;
    user.email = email;
    user.defaultChannel = channelName;
    GBServer.globals.users[user.userId] = user;
    if (user.changed()) {
      await user.save();
    }
    return user;
  }

  /**
   * Retrives a conversation reference from contact phone.
   */
  public async getConversationReference(phone: string): Promise<ConversationReference> {
    const options = <FindOptions>{ rejectOnEmpty: true, where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    return JSON.parse(user.conversationReference);
  }

  /**
   * Updates a conversation reference from contact phone.
   */
  public async updateConversationReference(phone: string, conversationReference: string) {
    const options = <FindOptions>{ where: { phone: phone } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    GBServer.globals.users[user.userId] = user;
    return await user.save();
  }

  public async updateConversationReferenceById(userId: number, conversationReference: string) {
    const options = <FindOptions>{ where: { userId: userId } };
    const user = await GuaribasUser.findOne(options);

    user.conversationReference = conversationReference;
    GBServer.globals.users[user.userId] = user;
    return await user.save();
  }

  public async updateUserLocale(userId: number, locale: any): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userId: userId
      }
    });
    user.locale = locale;
    GBServer.globals.users[user.userId] = user;
    return await user.save();
  }

  public async updateUserHearOnDialog(userId: number, dialogName: string): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userId: userId
      }
    });
    user.hearOnDialog = dialogName;
    GBServer.globals.users[user.userId] = user;
    return await user.save();
  }

  public async updateUserInstance(userSystemId: string, instanceId: number): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userSystemId: userSystemId
      }
    });
    user.instanceId = instanceId;
    GBServer.globals.users[user.userId] = user;
    return await user.save();
  }

  /**
   * Finds and update user agent information to a next available person.
   */
  public async updateHumanAgent(
    userSystemId: string,
    instanceId: number,
    agentSystemId: string
  ): Promise<GuaribasUser> {
    const user = await GuaribasUser.findOne({
      where: {
        userSystemId: userSystemId,
        instanceId: instanceId
      }
    });

    if (agentSystemId === null && user.agentSystemId !== undefined) {
      const agent = await GuaribasUser.findOne({
        where: {
          userSystemId: user.agentSystemId
        }
      });

      if (agent !== null && agent !== undefined) {
        agent.agentMode = 'bot';
        agent.agentSystemId = null;
        await agent.save();
      }

      user.agentMode = 'bot';
      user.agentSystemId = null;
    } else {
      user.agentMode = 'human';
      user.agentSystemId = agentSystemId;
      const agent = await GuaribasUser.findOne({
        where: {
          userSystemId: agentSystemId
        }
      });

      agent.instanceId = user.instanceId;
      agent.agentMode = 'self';
      agent.agentSystemId = null;
      GBServer.globals.users[agent.userId] = user;
      await agent.save();
    }

    GBServer.globals.users[user.userId] = user;
    await user.save();

    return user;
  }

  public async isAgentSystemId(systemId: string): Promise<Boolean> {
    const user = await GuaribasUser.findOne({
      where: {
        userSystemId: systemId
      }
    });

    if (user === null) {
      throw new Error(`TRANSFER_TO phones must talk first to the bot before becoming an agent.`);
    }

    return user.agentMode === 'self';
  }

  public async assignHumanAgent(
    min: GBMinInstance,
    userSystemId: string,
    agentSystemId: string = null
  ): Promise<string> {
    if (!agentSystemId) {
      let list = min.core.getParam<string>(min.instance, 'Transfer To', process.env.TRANSFER_TO);

      if (list) {
        list = list.split(';');
      }

      await GBUtil.asyncForEach(list, async item => {
        if (item !== undefined && !agentSystemId && item !== userSystemId && !(await this.isAgentSystemId(item))) {
          agentSystemId = item;
        }
      });
    }
    GBLogEx.info(min, `Selected agentId: ${agentSystemId}`);
    await this.updateHumanAgent(userSystemId, min.instance.instanceId, agentSystemId);
    GBLogEx.info(min, `Updated agentId to: ${agentSystemId}`);

    return agentSystemId;
  }

  public async getUserFromId(instanceId: number, userId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        instanceId: instanceId,
        userId: userId
      }
    });
  }

  public async getUserFromSystemId(systemId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        userSystemId: systemId
      }
    });
  }

  public async getUserFromAgentSystemId(systemId: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        agentSystemId: systemId
      }
    });
  }

  public async getAllUsers(instanceId: number): Promise<GuaribasUser[]> {
    return await GuaribasUser.findAll({
      where: {
        instanceId: instanceId
      }
    });
  }

  public async getUserFromUsername(instanceId: number, username: string): Promise<GuaribasUser> {
    return await GuaribasUser.findOne({
      where: {
        instanceId: instanceId,
        userName: username
      }
    });
  }

  /**
   * Get a dynamic param from user. Dynamic params are defined in .gbdialog SET
   * variables and other semantics during conversation.
   *
   * @param name Name of param to get from instance.
   * @param defaultValue Value returned when no param is defined.
   */
  public getParam<T>(user: GuaribasUser, name: string, defaultValue?: T): any {
    let value = null;
    if (user.params) {
      const params = JSON.parse(user.params);
      value = params ? params[name] : defaultValue;
    }
    if (typeof defaultValue === 'boolean') {
      return new Boolean(value ? value.toString().toLowerCase() === 'true' : defaultValue);
    }
    if (typeof defaultValue === 'string') {
      return value ? value : defaultValue;
    }
    if (typeof defaultValue === 'number') {
      return new Number(value ? value : defaultValue ? defaultValue : 0);
    }

    if (user['dataValues'] && !value) {
      value = user['dataValues'][name];
      if (value === null) {
        switch (name) {
          case 'language':
            value = 'en';
            break;
        }
      }
    }

    return value;
  }
  /**
   * Saves user instance object to the storage handling
   * multi-column JSON based store 'params' field.
   */
  public async setParam(userId: number, name: string, value: any) {
    const options = { where: {} };
    options.where = { userId: userId };
    let user = await GuaribasUser.findOne(options);
    // tslint:disable-next-line:prefer-object-spread
    let obj = JSON.parse(user.params);
    if (!obj) {
      obj = {};
    }
    obj[name] = value;
    user.params = JSON.stringify(obj);
    GBServer.globals.users[userId] = user;
    return await user.save();
  }
}
