//! Sequences (design-doc §5.6/§7): the handshake preserved event-for-event.
//! Late stimulus generation at the moment of grant — operands are filled
//! *between* start_item and finish_item, the ordering the book teaches.

use std::future::Future;
use std::pin::Pin;

use rustdv::prelude::*;

use crate::alu_item::{AluCommand, Ops};

/// Random operands across every op, `n_per_op` times (the book's random
/// test iterates all ops with random operands — coverage is guaranteed
/// by construction).
pub struct RandomSeq {
    pub n_per_op: usize,
    pub rng: Rng,
}

impl Sequence<AluCommand> for RandomSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for _ in 0..self.n_per_op {
                for op in Ops::ALL {
                    let mut cmd = AluCommand { a: 0, b: 0, op };
                    ctx.start_item(&mut cmd).await;
                    // Late generation: fill at grant time.
                    cmd.a = self.rng.u8();
                    cmd.b = self.rng.u8();
                    ctx.finish_item(cmd).await?;
                }
            }
            Ok(())
        })
    }
}

/// Maximum operands for every op (the book's MaxSeq: 0xFF op 0xFF).
pub struct MaxSeq;

impl Sequence<AluCommand> for MaxSeq {
    fn body<'a>(
        &'a mut self,
        mut ctx: SeqCtx<AluCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), SeqError>> + 'a>> {
        Box::pin(async move {
            for op in Ops::ALL {
                let mut cmd = AluCommand { a: 0xFF, b: 0xFF, op };
                ctx.start_item(&mut cmd).await;
                ctx.finish_item(cmd).await?;
            }
            Ok(())
        })
    }
}
