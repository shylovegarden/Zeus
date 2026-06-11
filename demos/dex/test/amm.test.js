const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("Zeus AMM DEX", function () {
  let amm;
  let owner;
  let addr1;
  let addr2;
  let tokenA;
  let tokenB;

  beforeEach(async function () {
    [owner, addr1, addr2] = await ethers.getSigners();

    // Deploy mock tokens
    const Token = await ethers.getContractFactory("MockERC20");
    tokenA = await Token.deploy("Token A", "TKA", ethers.utils.parseEther("1000000"));
    tokenB = await Token.deploy("Token B", "TKB", ethers.utils.parseEther("1000000"));

    // Deploy AMM
    const AMM = await ethers.getContractFactory("ZeusAMM");
    amm = await AMM.deploy(tokenA.address, tokenB.address);

    // Approve tokens
    await tokenA.approve(amm.address, ethers.constants.MaxUint256);
    await tokenB.approve(amm.address, ethers.constants.MaxUint256);
    await tokenA.connect(addr1).approve(amm.address, ethers.constants.MaxUint256);
    await tokenB.connect(addr1).approve(amm.address, ethers.constants.MaxUint256);
  });

  describe("Deployment", function () {
    it("Should set correct token addresses", async function () {
      expect(await amm.tokenA()).to.equal(tokenA.address);
      expect(await amm.tokenB()).to.equal(tokenB.address);
    });

    it("Should initialize with zero reserves", async function () {
      const reserveA = await amm.reserveA();
      const reserveB = await amm.reserveB();
      expect(reserveA).to.equal(0);
      expect(reserveB).to.equal(0);
    });
  });

  describe("Liquidity", function () {
    it("Should add initial liquidity", async function () {
      const amountA = ethers.utils.parseEther("1000");
      const amountB = ethers.utils.parseEther("1000");

      await expect(amm.addLiquidity(amountA, amountB, 0))
        .to.emit(amm, "LiquidityAdded")
        .withArgs(owner.address, amountA, amountB, ethers.utils.parseEther("1000"));

      expect(await amm.reserveA()).to.equal(amountA);
      expect(await amm.reserveB()).to.equal(amountB);
    });

    it("Should mint LP tokens proportional to liquidity", async function () {
      const amountA = ethers.utils.parseEther("1000");
      const amountB = ethers.utils.parseEther("1000");

      await amm.addLiquidity(amountA, amountB, 0);
      const lpBalance = await amm.balanceOf(owner.address);

      // LP tokens = sqrt(amountA * amountB)
      const expectedLP = ethers.utils.parseEther("1000");
      expect(lpBalance).to.equal(expectedLP);
    });

    it("Should maintain ratio for subsequent deposits", async function () {
      // First deposit
      await amm.addLiquidity(
        ethers.utils.parseEther("1000"),
        ethers.utils.parseEther("1000"),
        0
      );

      // Second deposit with different ratio should adjust
      const depositA = ethers.utils.parseEther("500");
      const depositB = ethers.utils.parseEther("600");

      await amm.connect(addr1).addLiquidity(depositA, depositB, 0);

      // Verify reserves maintain ratio
      const reserveA = await amm.reserveA();
      const reserveB = await amm.reserveB();

      // Ratio should be 1:1 (1000+500 : 1000+500)
      const ratio = reserveA.mul(1000).div(reserveB);
      expect(ratio).to.be.closeTo(1000, 1);
    });

    it("Should remove liquidity", async function () {
      const amountA = ethers.utils.parseEther("1000");
      const amountB = ethers.utils.parseEther("1000");

      await amm.addLiquidity(amountA, amountB, 0);
      const lpTokens = await amm.balanceOf(owner.address);

      await expect(amm.removeLiquidity(lpTokens, 0, 0))
        .to.emit(amm, "LiquidityRemoved")
        .withArgs(owner.address, lpTokens, amountA, amountB);

      expect(await amm.balanceOf(owner.address)).to.equal(0);
    });
  });

  describe("Swaps", function () {
    beforeEach(async function () {
      // Add initial liquidity
      await amm.addLiquidity(
        ethers.utils.parseEther("10000"),
        ethers.utils.parseEther("10000"),
        0
      );

      // Fund addr1 with tokens
      await tokenA.transfer(addr1.address, ethers.utils.parseEther("1000"));
    });

    it("Should swap token A for token B", async function () {
      const swapAmount = ethers.utils.parseEther("100");
      const expectedOutput = ethers.utils.parseEther("99"); // ~1% fee

      const balanceBefore = await tokenB.balanceOf(addr1.address);

      await amm.connect(addr1).swapAForB(swapAmount, expectedOutput);

      const balanceAfter = await tokenB.balanceOf(addr1.address);
      expect(balanceAfter.sub(balanceBefore)).to.be.closeTo(
        expectedOutput,
        ethers.utils.parseEther("1")
      );
    });

    it("Should maintain constant product invariant", async function () {
      const kBefore = (await amm.reserveA()).mul(await amm.reserveB());

      const swapAmount = ethers.utils.parseEther("100");
      await amm.connect(addr1).swapAForB(swapAmount, 0);

      const kAfter = (await amm.reserveA()).mul(await amm.reserveB());

      // k should never decrease (fee accumulation)
      expect(kAfter).to.be.gte(kBefore);
    });

    it("Should charge correct fee", async function () {
      const swapAmount = ethers.utils.parseEther("1000");
      const expectedOutput = await amm.getAmountOut(swapAmount, await amm.reserveA(), await amm.reserveB());

      // 0.3% fee
      const expectedWithoutFee = swapAmount.mul(await amm.reserveB()).div(await amm.reserveA().add(swapAmount));
      const fee = expectedWithoutFee.sub(expectedOutput);
      const feePercentage = fee.mul(10000).div(expectedWithoutFee);

      expect(feePercentage).to.be.closeTo(30, 1); // 0.3%
    });

    it("Should revert on insufficient output", async function () {
      const swapAmount = ethers.utils.parseEther("100");
      const minOutput = ethers.utils.parseEther("1000"); // Too high

      await expect(
        amm.connect(addr1).swapAForB(swapAmount, minOutput)
      ).to.be.revertedWith("Insufficient output");
    });
  });

  describe("Price Oracle", function () {
    beforeEach(async function () {
      await amm.addLiquidity(
        ethers.utils.parseEther("1000"),
        ethers.utils.parseEther("2000"),
        0
      );
    });

    it("Should return correct price", async function () {
      const price = await amm.getPrice(true); // A to B
      // Price = reserveB / reserveA = 2000/1000 = 2
      expect(price).to.be.closeTo(
        ethers.utils.parseEther("2"),
        ethers.utils.parseEther("0.01")
      );
    });
  });

  describe("Security Properties", function () {
    it("Should handle reentrancy protection", async function () {
      // This test would verify that the contract is protected against reentrancy
      // The Zeus compiler should generate reentrancy-safe code
    });

    it("Should prevent integer overflow", async function () {
      // Zeus compiler adds overflow checks
      const hugeAmount = ethers.constants.MaxUint256;

      await expect(
        amm.addLiquidity(hugeAmount, hugeAmount, 0)
      ).to.be.reverted;
    });
  });
});
