# 分布式系统笔记



## 1 并行计算模型

PRAM

APRAM

BSP 模型：见 [Introduction to the Bulk Synchronous Parallel model](http://albert-jan.yzelman.net/education/parco14/A2.pdf) 和 [BSP Model](https://jwbuurlage.github.io/Bulk/bsp/)



### 作业一

在 BSP 模型上计算 n 个数的和，假定每个处理器上分配 $\frac{n}{p}$ 个数，首先每个处理器求其自己的局部和，然后在 d 叉树自下而上求和。

1. 求算法的时间复杂度？

   根据 BSP 模型的定义，令 $w$ 为每次执行一次加法所消耗的时间，$g$ 为每个超级步中最大数据传输时间，$l$ 为同步延迟时间，则该算法的复杂度为 $\lceil \frac{n}{p} \rceil w + \lceil\log_d p \rceil(dw + g + l)$。

2. 如何选择 $d$ ？

   将上式对 $d$ 求导，并令其导数为零，解方程得到的解为最优的 $d$。





## 2 并行计算性能评测

| 名称                                                | 符号                                                      |
| --------------------------------------------------- | --------------------------------------------------------- |
| 机器数                                              | $p$                                                       |
| 工作负载（给定问题的总计算量）= 串行分量 + 并行分量 | $W=W_s + W_p$                                             |
| 用 p 个机器的执行时间                               | $T_p$（$T_1$ 为串行执行时间）                             |
| 加速比                                              | $S_p=\frac{W}{W_s + W_p /p}$                              |
| 串行比例分量                                        | $f=\frac{W_s}{W}=\lim_{p\rightarrow \infin}\frac{1}{S_p}$ |



### 作业二

1. 两个 $N\times N$ 矩阵相乘，时间复杂度为 $CN^3$，其中 $C$ 为常数。在 $n$ 个节点的并行机上并行矩阵乘法时间为 $T_p=CN^3+bN^2/\sqrt{n}$，其中 $b$ 为另一常数，第一项为计算开销，第二项为通信开销。分别求固定负载和固定时间时的加速并讨论结果。

   - 固定负载的加速比为 $\frac{CN^3}{CN^3/n + bN^2/\sqrt{n}}$，当 $n$ 趋于正无穷时，加速比也趋于正无穷
   - 固定时间的加速比同样为 $\frac{CN^3}{CN^3+bN^2/\sqrt{n}}$

2. 一个在 $p$ 个处理器上运行的并行程序加速比是 $p-1$，根据 Amdahl 定律，串行分量为多少？

   解加速比的定义方程可得 $W_s =\frac{W_p}{p(p-2)}$。

