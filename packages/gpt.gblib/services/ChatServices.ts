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
import { pdfToPng, PngPageOutput } from 'pdf-to-png-converter';
import { DynamicStructuredTool } from "@langchain/core/tools";
import { WikipediaQueryRun } from "@langchain/community/tools/wikipedia_query_run";
import {
  BaseLLMOutputParser,
  OutputParserException,
} from "@langchain/core/output_parsers";
import { ChatGeneration, Generation } from "@langchain/core/outputs";
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBServer } from '../../../src/app.js';
import urlJoin from 'url-join';
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";


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

export class GBLLMOutputParser extends BaseLLMOutputParser<ExpectedOutput> {
  lc_namespace = ["langchain", "output_parsers"];

  private toolChain: RunnableSequence
  private documentChain: RunnableSequence;
  private min;

  constructor(min, toolChain: RunnableSequence, documentChain: RunnableSequence) {
    super();
    this.min = min;
    this.toolChain = toolChain;

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

    const naiveJSONFromText = (text) => {
      const match = text.match(/\{[\s\S]*\}/);
      if (!match) return null;

      try {
        return JSON.parse(match[0]);
      } catch {
        return null;
      }
    };

    if (result) {
      const metadata = naiveJSONFromText(result);

      if (metadata) {
        const {url} = await ChatServices.pdfPageAsImage(this.min, metadata.filename,
          metadata.page);
        result = `![alt text](${url})
         ${result}`;
      }
    }

    return result;
  }
}

export class ChatServices {
  
  public static async pdfPageAsImage(min, filename, pageNumber) {
    
    const gbaiName = DialogKeywords.getGBAIPath(min.botId, 'gbkb');
    const localName = Path.join('work', gbaiName, 'docs', filename);

    // Converts the PDF to PNG.

    const pngPages: PngPageOutput[] = await pdfToPng(localName, {
      disableFontFace: true,
      useSystemFonts: true,
      viewportScale: 2.0,
      pagesToProcess: [pageNumber],
      strictPagesToProcess: false,
      verbosityLevel: 0
    });


    // Prepare an image on cache and return the GBFILE information.

    if (pngPages.length > 0) {
      const buffer = pngPages[0].content;
      const gbaiName = DialogKeywords.getGBAIPath(min.botId, null);
      const localName = Path.join('work', gbaiName, 'cache', `img${GBAdminService.getRndReadableIdentifier()}.png`);
      const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', Path.basename(localName));
      Fs.writeFileSync(localName, buffer, { encoding: null });
      return { localName: localName, url: url, data: buffer };
    }
  }

  private static async getRelevantContext(
    vectorStore: HNSWLib,
    sanitizedQuestion: string,
    numDocuments: number = 10
  ): Promise<string> {

    if (sanitizedQuestion === '') {
      return '';
    }

    const documents = await vectorStore.similaritySearch(sanitizedQuestion, numDocuments);
    let output = '';

    await CollectionUtil.asyncForEach(documents, async (doc) => {

      const metadata = doc.metadata;
      const filename = Path.basename(metadata.source);
      const page = await ChatServices.findPageForTextInterval(doc.metadata.source,
        metadata.loc.lines.from, metadata.loc.lines.to);

      output = `${output}\n\n\n\nThe following context is coming from ${filename} at page: ${page}, 
      memorize this block among document information and return when you are refering this part of content:\n\n\n\n ${doc.pageContent} \n\n\n\n.`;
    });
    return output;
  }

  public static async findPageForTextInterval(pdfPath, startLine, endLine) {
    const data = new Uint8Array(Fs.readFileSync(pdfPath));
    const pdf = await getDocument({ data }).promise;

    // Loop através de cada página para encontrar o intervalo de texto
    for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const textContent = await page.getTextContent();
      const text = textContent.items.map(item => item['str']).join('\n');

      const lines = text.split('\n');
      const numLines = lines.length;

      // Verificar se o intervalo de texto está nesta página
      const nextPageLine = (i === pdf.numPages) ? numLines : endLine;
      if (startLine <= numLines && endLine >= 0 && startLine <= nextPageLine) {
        return i;
      }

      startLine -= numLines;
      endLine -= numLines;
    }

    return -1; // Intervalo de texto não encontrado
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
        You have access to the following set of tools. 
        Here are the names and descriptions for each tool:

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
        This is a sectioned context. 
        
        Very important: When answering, besides the answer, at the end of the message, return this information as JSON
        containing two fields file as PDF 'filename' and 'page' as the page number.
        Eg. of JSON generated: file: filename.pdf, page: 3.
        
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
          return `${systemPrompt} \n ${c ? 'Use this context to answer:\n' + c : 'answer just with user question.'}`;

        },
      },
      combineDocumentsPrompt,
      model,
      new GBLLMOutputParser(min, null, null)
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
      new GBLLMOutputParser(min, callToolChain, docsContext?.docstore?._docs.length > 0 ? combineDocumentsChain : null),
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
      new GBLLMOutputParser(min, callToolChain, docsContext?.docstore?._docs.length > 0 ? combineDocumentsChain : null),
      new StringOutputParser()
    ]);

    let result;


    // Choose the operation mode of answer generation, based on 
    // .gbot switch LLMMode and choose the corresponding chain.

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
    else if (LLMMode === "full") {

      throw new Error('Not implemented.'); // TODO: #407.
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

    const tool = new WikipediaQueryRun({
      topKResults: 3,
      maxDocContentLength: 4000,
    });
    functions.push(tool);

    return functions;
  }
}
