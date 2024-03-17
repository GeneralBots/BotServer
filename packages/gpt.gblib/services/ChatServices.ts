/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { HNSWLib } from '@langchain/community/vectorstores/hnswlib';
import { StringOutputParser } from "@langchain/core/output_parsers";
import { AIMessagePromptTemplate, ChatPromptTemplate, HumanMessagePromptTemplate, MessagesPlaceholder } from '@langchain/core/prompts';
import { RunnableSequence } from "@langchain/core/runnables";
import { convertToOpenAITool } from "@langchain/core/utils/function_calling";
import { ChatOpenAI } from "@langchain/openai";
import { GBLog, GBMinInstance } from 'botlib';
import * as Fs from 'fs';
import { jsonSchemaToZod } from "json-schema-to-zod";
import { BufferWindowMemory } from 'langchain/memory';
import Path from 'path';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { GuaribasSubject } from '../../kb.gbapp/models/index.js';
import { Serialized } from "@langchain/core/load/serializable";
import { BaseCallbackHandler } from "@langchain/core/callbacks/base";
import { DynamicStructuredTool } from "@langchain/core/tools";
import {
  BaseLLMOutputParser,
  OutputParserException,
} from "@langchain/core/output_parsers";
import { ChatGeneration, Generation } from "@langchain/core/outputs";
import { LunaryHandler } from "@langchain/community/callbacks/handlers/lunary";

export interface CustomOutputParserFields { }
export type ExpectedOutput = string;

function isChatGeneration(
  llmOutput: ChatGeneration | Generation
): llmOutput is ChatGeneration {
  return "message" in llmOutput;
}

class CustomHandler extends BaseCallbackHandler {
  name = "custom_handler";

  handleLLMNewToken(token: string) {
    GBLog.info(`LLM: token: ${JSON.stringify(token)}`);
  }

  handleLLMStart(llm: Serialized, _prompts: string[]) {
    GBLog.info(`LLM: handleLLMStart ${JSON.stringify(llm)}, Prompts: ${_prompts.join('\n')}`);
  }

  handleChainStart(chain: Serialized) {
    GBLog.info(`LLM: handleChainStart: ${JSON.stringify(chain)}`);
  }

  handleToolStart(tool: Serialized) {
    GBLog.info(`LLM: handleToolStart: ${JSON.stringify(tool)}`);
  }
}

const logHandler = new CustomHandler();

export class CustomLLMOutputParser extends BaseLLMOutputParser<ExpectedOutput> {
  lc_namespace = ["langchain", "output_parsers"];

  private toolChain: RunnableSequence
  private documentChain: RunnableSequence;

  constructor(toolChain: RunnableSequence, documentChain: RunnableSequence) {
    super();
    this.toolChain = toolChain;
    this.documentChain = documentChain;
  }

  async parseResult(
    llmOutputs: ChatGeneration[] | Generation[]
  ): Promise<ExpectedOutput> {
    if (!llmOutputs.length) {
      throw new OutputParserException(
        "Output parser did not receive any generations."
      );
    }
    let result;

    if (llmOutputs[0]['message'].lc_kwargs.additional_kwargs.tool_calls) {
      return this.toolChain.invoke({ func: llmOutputs[0]['message'].lc_kwargs.additional_kwargs.tool_calls });
    }

    if (isChatGeneration(llmOutputs[0])) {
      result = llmOutputs[0].message.content;
    } else {
      result = llmOutputs[0].text;
    }

    return this.documentChain ? this.documentChain.invoke(result) : result;

  }
}

export class ChatServices {

  private static async getRelevantContext(
    vectorStore: HNSWLib,
    sanitizedQuestion: string,
    numDocuments: number = 10
  ): Promise<string> {
    if (sanitizedQuestion === '') {
      return '';
    }

    const documents = await vectorStore.similaritySearch(sanitizedQuestion, numDocuments);
    return documents
      .map((doc) => doc.pageContent)
      .join(', ')
      .trim()
      .replaceAll('\n', ' ');
  }

  /**
   * Generate text
   *
   * CONTINUE keword.
   *
   * result = CONTINUE text
   *
   */
  public static async continue(min: GBMinInstance, question: string, chatId) {

  }

  private static memoryMap = {};
  public static userSystemPrompt = {};

  public static async answerByGPT(min: GBMinInstance, user, pid,
    question: string,
    searchScore: number,
    subjects: GuaribasSubject[]
  ) {

    if (!process.env.OPENAI_API_KEY) {
      return { answer: undefined, questionId: 0 };
    }

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );
    const LLMMode = min.core.getParam(
      min.instance,
      'Answer Mode', 'direct'
    );
    const docsContext = min['vectorStore'];

    if (!this.memoryMap[user.userSystemId]) {
      this.memoryMap[user.userSystemId] = new BufferWindowMemory({
        returnMessages: true,
        memoryKey: 'chat_history',
        inputKey: 'input',
        k: 2,
      })
    }
    const memory = this.memoryMap[user.userSystemId];
    const systemPrompt = this.userSystemPrompt[user.userSystemId];

    const model = new ChatOpenAI({
      openAIApiKey: process.env.OPENAI_API_KEY,
      modelName: "gpt-3.5-turbo-0125",
      temperature: 0,
      callbacks: [logHandler],
    });


    let tools = await ChatServices.getTools(min);
    let toolsAsText = ChatServices.getToolsAsText(tools);

    const modelWithTools = model.bind({
      tools: tools.map(convertToOpenAITool)
    });

    const questionGeneratorTemplate = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        `
        Answer the question without calling any tool, but if there is a need to call:
         You have access to the following set of tools. Here are the names and descriptions for each tool:
          ${toolsAsText}

          Do not use any previous tools output in the chat_history. 
        `
      ),
      new MessagesPlaceholder("chat_history"),
      AIMessagePromptTemplate.fromTemplate(`Follow Up Input: {question}
    Standalone question:`),
    ]);

    const toolsResultPrompt = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        `The tool just returned value in last call. Using {chat_history}
        rephrase the answer to the user using this tool output.
        `
      ),
      new MessagesPlaceholder("chat_history"),
      AIMessagePromptTemplate.fromTemplate(`Tool output: {tool_output} 
    Standalone question:`),
    ]);

    const combineDocumentsPrompt = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        `
        \n\n{context}\n\n

        And using \n\n{chat_history}\n\n
        rephrase the answer to the user using this context already spoken.
        If you don't know the answer, just say that you don't know, don't try to make up an answer.
        Use the following pieces, if any, of context to answer the question at the end. 

        `
      ),
      new MessagesPlaceholder("chat_history"),
      HumanMessagePromptTemplate.fromTemplate("Question: {question}"),
    ]);

    const callToolChain = RunnableSequence.from([
      {
        tool_output: async (output: object) => {

          const name = output['func'][0].function.name;
          const args = JSON.parse(output['func'][0].function.arguments);
          GBLog.info(`Running .gbdialog '${name}' as GPT tool...`);
          const pid = GBVMService.createProcessInfo(null, min, 'gpt', null);

          return await GBVMService.callVM(name, min, false, pid, false, args);
        },
        chat_history: async () => {
          const { chat_history } = await memory.loadMemoryVariables({});
          return chat_history;
        },

      },
      toolsResultPrompt,
      model,
      new StringOutputParser()
    ]);

    const combineDocumentsChain = RunnableSequence.from([
      {
        question: (question: string) => question,
        chat_history: async () => {
          const { chat_history } = await memory.loadMemoryVariables({});
          return chat_history;
        },
        context: async (output: string) => {
          const c = await ChatServices.getRelevantContext(docsContext, output);
          return  `${systemPrompt} \n ${c ? 'Use this context to answer:\n' + c: 'answer just with user question.'}`;

        },
      },
      combineDocumentsPrompt,
      model,
      new StringOutputParser()
    ]);

    const conversationalQaChain = RunnableSequence.from([
      {
        question: (i: { question: string }) => i.question,
        chat_history: async () => {
          const { chat_history } = await memory.loadMemoryVariables({});
          return chat_history;
        },
      },
      questionGeneratorTemplate,
      modelWithTools,
      new CustomLLMOutputParser(callToolChain, docsContext?.docstore?._docs.length > 0 ? combineDocumentsChain : null),
      new StringOutputParser()
    ]);

    const conversationalToolChain = RunnableSequence.from([
      {
        question: (i: { question: string }) => i.question,
        chat_history: async () => {
          const { chat_history } = await memory.loadMemoryVariables({});
          return chat_history;
        },
      },
      questionGeneratorTemplate,
      modelWithTools,
      new CustomLLMOutputParser(callToolChain, docsContext?.docstore?._docs.length > 0 ? combineDocumentsChain : null),
      new StringOutputParser()
    ]);

    let result;

    if (LLMMode === "direct") {
      result = await (tools.length > 0 ? modelWithTools : model).invoke(`
      ${systemPrompt}
      
      ${question}`);

      result = result.content;
    }
    else if (LLMMode === "document") {
      result = await combineDocumentsChain.invoke(question);

    } else if (LLMMode === "function") {

      result = await conversationalToolChain.invoke({
        question,
      });
    }
    else {
      GBLog.info(`Invalid Answer Mode in Config.xlsx: ${LLMMode}.`);
    }

    await memory.saveContext(
      {
        input: question,
      },
      {
        output: result,
      }
    );

    GBLog.info(`GPT Result: ${result.toString()}`);
    return { answer: result.toString(), questionId: 0 };


  }

  private static getToolsAsText(tools) {
    return Object.keys(tools)
      .map((toolname) => `- ${tools[toolname].name}: ${tools[toolname].description}`)
      .join("\n");
  }

  private static async getTools(min: GBMinInstance) {
    let functions = [];

    // Adds .gbdialog as functions if any to GPT Functions.
    await CollectionUtil.asyncForEach(Object.keys(min.scriptMap), async (script) => {


      const path = DialogKeywords.getGBAIPath(min.botId, "gbdialog", null);
      const jsonFile = Path.join('work', path, `${script}.json`);

      if (Fs.existsSync(jsonFile) && script.toLowerCase() !== 'start.vbs') {

        const funcJSON = JSON.parse(Fs.readFileSync(jsonFile, 'utf8'));
        const funcObj = funcJSON?.function;

        if (funcObj) {

          // TODO: Use ajv.
          funcObj.schema = eval(jsonSchemaToZod(funcObj.parameters));
          functions.push(new DynamicStructuredTool(funcObj));
        }

      }

    });

    return functions;
  }
}
