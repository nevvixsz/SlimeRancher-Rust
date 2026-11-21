using System.Collections.Generic;
using UnityEngine;

namespace SlimeDevConsole.Commands
{
    internal sealed class FlyCommand : DevCommandBase
    {
        public FlyCommand()
            : base("полет",
                "полет [множитель_скорости]",
                "Включает/выключает свободный полёт сквозь стены с настройкой скорости (напр. 'полет 5').")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            if (args != null && args.Length >= 1 && CommandHelpers.TryParseFloat(args[0], out float spd))
            {
                DevState.ToggleFly(spd);
                CommandHelpers.LogSuccess(string.Format("Полёт ВКЛ (скорость x{0:0.##}). Повторный вызов 'полет' выключит режим.", DevState.FlySpeedMultiplier));
                return true;
            }

            DevState.ToggleFly(null);
            CommandHelpers.LogSuccess("Полёт: " + (DevState.FlyMode ? string.Format("ВКЛ (скорость x{0:0.##})", DevState.FlySpeedMultiplier) : "ВЫКЛ"));
            return true;
        }
    }

    internal sealed class SpeedCommand : DevCommandBase
    {
        public SpeedCommand()
            : base("скорость",
                "скорость <множитель>",
                "Устанавливает множитель скорости бега и прыжка игрока (1 = стандарт).")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            if (args == null || args.Length == 0 || !CommandHelpers.TryParseFloat(args[0], out float mult))
            {
                CommandHelpers.Log("Текущая скорость бега: x" + DevState.WalkSpeedMultiplier.ToString("0.##") + ". Использование: скорость 2.5");
                return true;
            }

            DevState.SetWalkSpeed(mult);
            CommandHelpers.LogSuccess("Скорость бега установлена: x" + DevState.WalkSpeedMultiplier.ToString("0.##"));
            return true;
        }
    }

    internal sealed class GodCommand : DevCommandBase
    {
        public GodCommand()
            : base("бессмертие",
                "бессмертие [вкл|выкл]",
                "Режим бога: иммунитет к урону, таррам, радиации и падениям.")
        {
        }

        public override bool Execute(string[] args)
        {
            if (args != null && args.Length > 0)
            {
                string s = RussianCatalog.Normalize(args[0]);
                DevState.GodMode = (s == "вкл" || s == "on" || s == "1" || s == "да");
            }
            else
            {
                DevState.GodMode = !DevState.GodMode;
            }

            CommandHelpers.LogSuccess("Бессмертие (GodMode): " + (DevState.GodMode ? "ВКЛ" : "ВЫКЛ"));
            return true;
        }
    }

    internal sealed class InfEnergyCommand : DevCommandBase
    {
        public InfEnergyCommand()
            : base("энергия",
                "энергия [вкл|выкл]",
                "Бесконечная энергия для бега, джетпака и импульсной волны.")
        {
        }

        public override bool Execute(string[] args)
        {
            if (args != null && args.Length > 0)
            {
                string s = RussianCatalog.Normalize(args[0]);
                DevState.InfiniteEnergy = (s == "вкл" || s == "on" || s == "1" || s == "да");
            }
            else
            {
                DevState.InfiniteEnergy = !DevState.InfiniteEnergy;
            }

            CommandHelpers.LogSuccess("Бесконечная энергия: " + (DevState.InfiniteEnergy ? "ВКЛ" : "ВЫКЛ"));
            return true;
        }
    }

    internal sealed class InfVacCommand : DevCommandBase
    {
        public InfVacCommand()
            : base("вакуум",
                "вакуум [вкл|выкл]",
                "Бесконечные предметы/вода в вакуумпаке (не расходуются при выстреле).")
        {
        }

        public override bool Execute(string[] args)
        {
            if (args != null && args.Length > 0)
            {
                string s = RussianCatalog.Normalize(args[0]);
                DevState.InfiniteVac = (s == "вкл" || s == "on" || s == "1" || s == "да");
            }
            else
            {
                DevState.InfiniteVac = !DevState.InfiniteVac;
            }

            CommandHelpers.LogSuccess("Бесконечный вакуумпак: " + (DevState.InfiniteVac ? "ВКЛ" : "ВЫКЛ"));
            return true;
        }
    }

    internal sealed class MoneyCommand : DevCommandBase
    {
        public MoneyCommand()
            : base("деньги",
                "деньги <сумма>",
                "Добавляет (или отнимает при минусе) ньюбаксы на счёт игрока.")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            if (args == null || args.Length == 0 || !CommandHelpers.TryParseInt(args[0], out int amount))
            {
                CommandHelpers.Log("Текущий баланс: " + ctx.PlayerState.GetCurrency() + " ньюбаксов. Использование: деньги 50000");
                return true;
            }

            if (amount >= 0)
            {
                ctx.PlayerState.AddCurrency(amount, PlayerState.CoinsType.NORM);
                CommandHelpers.LogSuccess(string.Format("Начислено +{0}. Баланс: {1}.", amount, ctx.PlayerState.GetCurrency()));
            }
            else
            {
                ctx.PlayerState.SpendCurrency(-amount, true);
                CommandHelpers.LogSuccess(string.Format("Списано {0}. Баланс: {1}.", -amount, ctx.PlayerState.GetCurrency()));
            }
            return true;
        }
    }

    internal sealed class HealCommand : DevCommandBase
    {
        public HealCommand()
            : base("лечить",
                "лечить",
                "Полностью восстанавливает здоровье и энергию, снимает радиацию.")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            ctx.PlayerState.SetHealth(ctx.PlayerState.GetMaxHealth());
            ctx.PlayerState.SetEnergy(ctx.PlayerState.GetMaxEnergy());
            ctx.PlayerState.SetRad(0);
            CommandHelpers.LogSuccess("Здоровье и энергия полностью восстановлены, радиация очищена.");
            return true;
        }
    }

    internal sealed class KeyCommand : DevCommandBase
    {
        public KeyCommand()
            : base("ключ",
                "ключ [кол-во]",
                "Выдаёт слаймовые ключи для древних ворот.")
        {
        }

        public override bool Execute(string[] args)
        {
            SceneContext ctx;
            GameContext gctx;
            if (!CommandHelpers.EnsureInGame(out ctx, out gctx))
            {
                return true;
            }

            int count = 1;
            if (args != null && args.Length >= 1 && CommandHelpers.TryParseInt(args[0], out int parsed))
            {
                count = Mathf.Clamp(parsed, 1, 20);
            }

            for (int i = 0; i < count; i++)
            {
                ctx.PlayerState.KeyAdded();
            }

            CommandHelpers.LogSuccess(string.Format("Выдано ключей: +{0} (всего в запасе: {1}).", count, ctx.PlayerState.GetKeys()));
            return true;
        }
    }
}
