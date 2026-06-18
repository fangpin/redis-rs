const translations = {
  en: {
    meta: {
      title: "redis-rs | Build Your Own Redis in Rust",
      description:
        "A Redis clone in Rust with RESP parsing, RDB loading, replication, streams, and transactions.",
      languageSwitcher: "Language switcher",
    },
    nav: {
      overview: "Detailed Docs",
      overviewHref: "/redis-rs/docs/overview/",
      github: "GitHub",
    },
    hero: {
      eyebrow: "Protocol. Persistence. Replication.",
      title: "Build Your Own Redis in Rust",
      description:
        "A toy Redis server in Rust that parses RESP, restores from RDB, replicates leader-follower state, supports streams, and queues transactional commands.",
      primaryCta: "View on GitHub",
      secondaryCta: "Quick Start",
      docsCta: "Detailed Docs",
      docsHref: "/redis-rs/docs/overview/",
    },
    docsEntry: {
      eyebrow: "Implementation docs",
      title: "Go straight to the detailed implementation docs",
      primaryCta: "Docs overview",
      primaryHref: "/redis-rs/docs/overview/",
      runtime: {
        kicker: "Runtime",
        title: "Server runtime",
        body: "Process entry, configuration, listener lifecycle, and shared server state.",
        href: "/redis-rs/docs/server-runtime/",
      },
      protocol: {
        kicker: "Protocol",
        title: "RESP and storage",
        body: "Wire format, parser, string storage, expiry, and stream containers.",
        href: "/redis-rs/docs/resp-protocol/",
      },
      replication: {
        kicker: "Sync",
        title: "RDB and replication",
        body: "Snapshot parsing, handshake flow, full sync, and replica fan-out.",
        href: "/redis-rs/docs/replication-flow/",
      },
      commands: {
        kicker: "Behavior",
        title: "Commands and transactions",
        body: "Dispatch, stream semantics, transaction replay, and the full chapter map.",
        href: "/redis-rs/docs/command-execution/",
      },
    },
    metrics: {
      protocol: "Protocol",
      persistence: "Persistence",
      replication: "Replication",
      streams: "Queues",
    },
    capabilities: {
      eyebrow: "Core capabilities",
      title: "Built around real Redis subsystems",
      protocol: {
        title: "Protocol",
        body: "Parses Redis protocol messages and routes commands through the server runtime.",
      },
      persistence: {
        title: "Persistence",
        body: "Loads database state from RDB files and restores key metadata.",
      },
      replication: {
        title: "Replication",
        body: "Bootstraps replica handshake, full sync, and replicated write flow.",
      },
      streams: {
        title: "Streams and transactions",
        body: "Supports stream operations and queued transactional execution patterns.",
      },
    },
    quickstart: {
      eyebrow: "Run it",
      title: "Start fast, then probe with redis-cli",
      masterLabel: "Master",
      replicaLabel: "Replica",
      probeLabel: "Probe",
    },
    architecture: {
      eyebrow: "Implementation map",
      title: "How the runtime is assembled",
      protocol: {
        title: "Protocol ingestion",
        body: "RESP parsing and request framing for incoming client and replication traffic.",
      },
      command: {
        title: "Command routing",
        body: "Connection handling, command dispatch, and transaction orchestration.",
      },
      storage: {
        title: "State and restore",
        body: "In-memory storage backed by RDB parsing and initial database hydration.",
      },
      replication: {
        title: "Replication flow",
        body: "Replica-side handshake, sync start, and downstream command consumption.",
      },
    },
    docs: {
      eyebrow: "Implementation docs",
      title: "Follow the code by subsystem",
      openChapter: "Open chapter",
      openOverview: "Open overview",
      runtime: {
        title: "Server runtime",
        body: "Process entry, configuration, listener lifecycle, and shared server state.",
        href: "/redis-rs/docs/server-runtime/",
      },
      persistence: {
        title: "RESP and storage",
        body: "RESP parsing and encoding, string storage, expiry model, and stream containers.",
        href: "/redis-rs/docs/resp-protocol/",
      },
      commands: {
        title: "Persistence and replication",
        body: "RDB parser internals, handshake flow, full sync, and replica fan-out.",
        href: "/redis-rs/docs/replication-flow/",
      },
      overview: {
        title: "Command surface",
        body: "Command dispatch, stream semantics, transaction replay, and the full chapter map.",
        href: "/redis-rs/docs/overview/",
      },
    },
    footer: {
      eyebrow: "Source and docs",
      title: "Inspect the code, then dive deeper",
      github: "Open GitHub Repository",
      readmeEn: "Read README",
      readmeZh: "Read Chinese README",
    },
  },
  zh: {
    meta: {
      title: "redis-rs | 用 Rust 构建自己的 Redis",
      description:
        "一个用 Rust 编写的 toy Redis server，支持 RESP 解析、RDB 恢复、主从复制、Stream，以及事务命令排队执行。",
      languageSwitcher: "语言切换",
    },
    nav: {
      overview: "详细文档",
      overviewHref: "/redis-rs/zh/docs/overview/",
      github: "GitHub",
    },
    hero: {
      eyebrow: "协议。持久化。复制。",
      title: "用 Rust 构建自己的 Redis",
      description:
        "一个用 Rust 编写的 toy Redis server，支持 RESP 解析、RDB 恢复、主从复制、Stream，以及事务命令排队执行。",
      primaryCta: "查看 GitHub 仓库",
      secondaryCta: "快速开始",
      docsCta: "详细文档",
      docsHref: "/redis-rs/zh/docs/overview/",
    },
    docsEntry: {
      eyebrow: "实现文档",
      title: "直接进入详细实现文档",
      primaryCta: "文档总览",
      primaryHref: "/redis-rs/zh/docs/overview/",
      runtime: {
        kicker: "运行时",
        title: "运行时与服务端",
        body: "进程入口、配置模型、监听生命周期，以及共享服务端状态。",
        href: "/redis-rs/zh/docs/server-runtime/",
      },
      protocol: {
        kicker: "协议层",
        title: "RESP 与存储",
        body: "线协议解析与编码、字符串存储、过期模型，以及 stream 容器结构。",
        href: "/redis-rs/zh/docs/resp-protocol/",
      },
      replication: {
        kicker: "同步链路",
        title: "RDB 与复制",
        body: "快照解析、握手流程、全量同步，以及 replica fan-out。",
        href: "/redis-rs/zh/docs/replication-flow/",
      },
      commands: {
        kicker: "行为层",
        title: "命令与事务",
        body: "命令分发、stream 语义、事务重放，以及完整章节地图。",
        href: "/redis-rs/zh/docs/command-execution/",
      },
    },
    metrics: {
      protocol: "协议",
      persistence: "持久化",
      replication: "复制",
      streams: "队列",
    },
    capabilities: {
      eyebrow: "核心能力",
      title: "围绕真实 Redis 子系统构建",
      protocol: {
        title: "协议处理",
        body: "解析 Redis 协议消息，并把命令路由到服务端运行时。",
      },
      persistence: {
        title: "持久化",
        body: "从 RDB 文件加载数据库状态，并恢复键的元数据。",
      },
      replication: {
        title: "主从复制",
        body: "完成 replica 握手、全量同步和写命令复制流程。",
      },
      streams: {
        title: "流与事务",
        body: "支持 Stream 相关操作，以及事务命令排队和执行模式。",
      },
    },
    quickstart: {
      eyebrow: "开始运行",
      title: "先跑起来，再用 redis-cli 探测",
      masterLabel: "主节点",
      replicaLabel: "从节点",
      probeLabel: "探测命令",
    },
    architecture: {
      eyebrow: "实现映射",
      title: "运行时如何拼装起来",
      protocol: {
        title: "协议入口",
        body: "负责 RESP 解析，以及客户端与复制流量的请求分帧。",
      },
      command: {
        title: "命令路由",
        body: "负责连接处理、命令分发，以及事务编排。",
      },
      storage: {
        title: "状态与恢复",
        body: "以内存存储为核心，并由 RDB 解析完成初始数据装载。",
      },
      replication: {
        title: "复制链路",
        body: "负责 replica 侧握手、同步启动和下游命令消费。",
      },
    },
    docs: {
      eyebrow: "实现文档",
      title: "按子系统阅读代码",
      openChapter: "打开章节",
      openOverview: "打开总览",
      runtime: {
        title: "运行时与服务端",
        body: "进程入口、配置模型、监听生命周期，以及共享服务端状态。",
        href: "/redis-rs/zh/docs/server-runtime/",
      },
      persistence: {
        title: "RESP 与存储",
        body: "RESP 解析与编码、字符串存储、过期模型，以及 stream 容器结构。",
        href: "/redis-rs/zh/docs/resp-protocol/",
      },
      commands: {
        title: "持久化与复制",
        body: "RDB 解析器内部、握手流程、全量同步，以及 replica fan-out。",
        href: "/redis-rs/zh/docs/replication-flow/",
      },
      overview: {
        title: "命令执行面",
        body: "命令分发、Stream 语义、事务重放，以及完整章节地图。",
        href: "/redis-rs/zh/docs/overview/",
      },
    },
    footer: {
      eyebrow: "源码与文档",
      title: "先看代码，再继续深入",
      github: "打开 GitHub 仓库",
      readmeEn: "查看英文 README",
      readmeZh: "查看中文 README",
    },
  },
};

const LANG_STORAGE_KEY = "redis-rs-homepage-language";

function getValue(source, path) {
  return path.split(".").reduce((acc, key) => acc && acc[key], source);
}

function applyLanguage(lang) {
  const dictionary = translations[lang] || translations.en;
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  document.title = dictionary.meta.title;

  document.querySelectorAll("[data-i18n]").forEach((node) => {
    const value = getValue(dictionary, node.dataset.i18n);
    if (typeof value === "string") {
      node.textContent = value;
    }
  });

  document.querySelectorAll("[data-i18n-content]").forEach((node) => {
    const value = getValue(dictionary, node.dataset.i18nContent);
    if (typeof value === "string") {
      node.setAttribute("content", value);
    }
  });

  document.querySelectorAll("[data-i18n-aria-label]").forEach((node) => {
    const value = getValue(dictionary, node.dataset.i18nAriaLabel);
    if (typeof value === "string") {
      node.setAttribute("aria-label", value);
    }
  });

  document.querySelectorAll("[data-i18n-href]").forEach((node) => {
    const value = getValue(dictionary, node.dataset.i18nHref);
    if (typeof value === "string") {
      node.setAttribute("href", value);
    }
  });

  document.querySelectorAll(".lang-btn").forEach((button) => {
    const active = button.dataset.lang === lang;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  });

  window.localStorage.setItem(LANG_STORAGE_KEY, lang);
}

function preferredLanguage() {
  const saved = window.localStorage.getItem(LANG_STORAGE_KEY);
  return saved === "zh" ? "zh" : "en";
}

document.addEventListener("DOMContentLoaded", () => {
  applyLanguage(preferredLanguage());

  document.querySelectorAll(".lang-btn").forEach((button) => {
    button.addEventListener("click", () => {
      applyLanguage(button.dataset.lang === "zh" ? "zh" : "en");
    });
  });
});
