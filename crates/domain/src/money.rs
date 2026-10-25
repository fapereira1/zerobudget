use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    pub amount: i64,
}

impl Money {
    pub fn from_cents(amount: i64) -> Result<Money, DomainError> {
        if amount < 0 {
            return Err(DomainError::InvalidAmount);
        }
        Ok(Self { amount })
    }

    pub fn to_cents(&self) -> i64 {
        self.amount
    }

    pub fn zero() -> Money {
        Self { amount: 0 }
    }

    pub fn to_real(&self) -> f64 {
        self.amount as f64 / 100.0
    }
}

impl std::ops::Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            amount: self.amount + other.amount,
        }
    }
}

impl std::ops::Sub for Money {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            amount: self.amount - other.amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_money_valid() {
        let money = Money::from_cents(1000).unwrap();
        assert_eq!(money.to_cents(), 1000);
    }

    #[test]
    fn test_create_money_negative_fails() {
        let result = Money::from_cents(-100);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DomainError::InvalidAmount);
    }

    #[test]
    fn test_zero() {
        let money = Money::zero();
        assert_eq!(money.to_cents(), 0);
    }

    #[test]
    fn test_add() {
        let money1 = Money::from_cents(1000).unwrap(); // R$ 10
        let money2 = Money::from_cents(500).unwrap(); // R$ 5
        let result = money1 + money2;
        assert_eq!(result.to_cents(), 1500); // R$ 15
    }

    #[test]
    fn test_sub() {
        let money1 = Money::from_cents(1000).unwrap(); // R$ 10
        let money2 = Money::from_cents(300).unwrap(); // R$ 3
        let result = money1 - money2;
        assert_eq!(result.to_cents(), 700); // R$ 7
    }

    #[test]
    fn test_to_real() {
        let money = Money::from_cents(1000).unwrap(); // 1000 centavos = R$ 10
        assert_eq!(money.to_real(), 10.0);
    }
}
