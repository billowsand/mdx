%% official.cls
%% 公文 LaTeX class for mdx
%% 2026/04/26 v1.0
%%
%% 视觉目标：与 mdx 公文 docx emitter 输出一致（仿宋三号 / 1.5 倍行距 /
%% 2em 首行缩进 / 公文页边距 / 居中页脚 "— X —"）。
%%
%% 编号规则在 tex_official.rs 里完成，本类不调用 \section*/\subsection*；
%% 因此这里设置 secnumdepth=-2 屏蔽自动编号。

\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{official}[2026/04/26 v1.0 mdx 公文 markdown class]

%% ============================================================
%% 基类：ctexart（无章）
%% ============================================================
%% ctexart 的 zihao 类选项只接受 {5, -4}；正文 三号 在 \AtBeginDocument 中设置。
\LoadClass[a4paper,UTF8,zihao=-4]{ctexart}

%% ============================================================
%% 包加载
%% ============================================================
\RequirePackage{geometry}
\RequirePackage{fancyhdr}
\RequirePackage{array}
\RequirePackage{etoolbox}
\RequirePackage{xcolor}

%% 关闭自动节编号
\setcounter{secnumdepth}{-2}

%% ============================================================
%% 页面
%% ============================================================
\geometry{
  a4paper,
  top=3.7cm,
  bottom=3.5cm,
  left=2.8cm,
  right=2.6cm,
  footskip=1.2cm
}

%% ============================================================
%% 字体（带 \IfFontExistsTF 兜底）
%% ============================================================

%% 标题：方正小标宋简体 → FZXiaoBiaoSong-B05 → SimSun
\IfFontExistsTF{方正小标宋简体}{%
  \setCJKfamilyfont{xbsong}{方正小标宋简体}%
}{%
  \IfFontExistsTF{FZXiaoBiaoSong-B05}{%
    \setCJKfamilyfont{xbsong}{FZXiaoBiaoSong-B05}%
  }{%
    \setCJKfamilyfont{xbsong}{SimSun}%
    \ClassWarning{official}{字体 '方正小标宋简体' 不可用，回退到 SimSun}%
  }%
}
\NewDocumentCommand{\xbsong}{}{\CJKfamily{xbsong}}

%% 二级：黑体 → SimHei → FZHei-B01
\IfFontExistsTF{黑体}{%
  \setCJKfamilyfont{hei}{黑体}%
}{%
  \IfFontExistsTF{SimHei}{%
    \setCJKfamilyfont{hei}{SimHei}%
  }{%
    \setCJKfamilyfont{hei}{FZHei-B01}%
    \ClassWarning{official}{字体 '黑体' 不可用，回退到 FZHei-B01}%
  }%
}
\NewDocumentCommand{\hei}{}{\CJKfamily{hei}}

%% 三级：楷体_GB2312 → KaiTi_GB2312 → KaiTi → FZKai-Z03
\IfFontExistsTF{楷体_GB2312}{%
  \setCJKfamilyfont{kai}{楷体_GB2312}%
}{%
  \IfFontExistsTF{KaiTi_GB2312}{%
    \setCJKfamilyfont{kai}{KaiTi_GB2312}%
  }{%
    \IfFontExistsTF{KaiTi}{%
      \setCJKfamilyfont{kai}{KaiTi}%
    }{%
      \setCJKfamilyfont{kai}{FZKai-Z03}%
      \ClassWarning{official}{字体 '楷体_GB2312' 不可用，回退到 FZKai-Z03}%
    }%
  }%
}
\NewDocumentCommand{\kai}{}{\CJKfamily{kai}}

%% 正文：仿宋_GB2312 → FangSong_GB2312 → FangSong → FZShuSong-Z01
\IfFontExistsTF{仿宋_GB2312}{%
  \setCJKfamilyfont{fs}{仿宋_GB2312}%
}{%
  \IfFontExistsTF{FangSong_GB2312}{%
    \setCJKfamilyfont{fs}{FangSong_GB2312}%
  }{%
    \IfFontExistsTF{FangSong}{%
      \setCJKfamilyfont{fs}{FangSong}%
    }{%
      \setCJKfamilyfont{fs}{FZShuSong-Z01}%
      \ClassWarning{official}{字体 '仿宋_GB2312' 不可用，回退到 FZShuSong-Z01}%
    }%
  }%
}
\NewDocumentCommand{\fs}{}{\CJKfamily{fs}}

%% ============================================================
%% 正文默认风格：仿宋 三号、1.5 倍行距、2em 首行缩进
%% ============================================================
\AtBeginDocument{\fs\zihao{3}}
\linespread{1.5}\selectfont
\setlength{\parindent}{2em}
\setlength{\parskip}{0pt}

%% ============================================================
%% 页脚 "— X —"，居中
%% ============================================================
\pagestyle{fancy}
\fancyhf{}
\fancyfoot[C]{\fs\zihao{-4}\textemdash\ \thepage\ \textemdash}
\renewcommand{\headrulewidth}{0pt}
\renewcommand{\footrulewidth}{0pt}

\endinput
